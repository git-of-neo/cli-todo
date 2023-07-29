use event::{Event, Key};
use sqlx::{self, FromRow, SqlitePool};
use std::fmt;
use std::io::{stdin, stdout, Write};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, color, cursor, event, style};
use tokio::time::{sleep, Duration};

#[derive(FromRow)]
struct Todo {
    id: i32,
    content: String,
}

impl fmt::Display for Todo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Todo : {}", self.content)
    }
}

async fn get_todos(pool: &SqlitePool) -> Result<Vec<Todo>, sqlx::Error> {
    sqlx::query_as::<_, Todo>("SELECT id, content FROM Todo WHERE active=1;")
        .fetch_all(pool)
        .await
}

async fn add_todo(pool: &SqlitePool, content: String) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO Todo (content) VALUES (?)")
        .bind(content)
        .execute(pool)
        .await?;
    Ok(())
}

async fn list_todos(todos: &Vec<Todo>) -> Result<u16, sqlx::Error> {
    println!(
        "{}{}{}Welcome!{}",
        style::Bold,
        clear::All,
        cursor::Goto(1, 1),
        style::Reset
    );
    println!("{}", cursor::Goto(1, 1));
    let mut line = 2u16;
    for t in todos {
        println!("{}{}", t, cursor::Goto(1, line));
        line += 1;
    }
    Ok(line)
}

async fn list_view<W: Write>(pool: &SqlitePool, stdout: &mut W) -> Result<(), sqlx::Error> {
    let mut todos = get_todos(&pool).await?;
    let mut line = list_todos(&todos).await?;

    let stdin = stdin();
    for c in stdin.events() {
        match c.unwrap() {
            Event::Key(Key::Char('\n')) => {
                if line < 2 {
                    continue;
                }
                let i = line as usize - 2;
                if i >= todos.len() {
                    continue;
                }
                sqlx::query(
                    "
                    UPDATE Todo
                    SET active = 0
                    WHERE id = (?);
                ",
                )
                .bind(todos[i].id)
                .execute(pool)
                .await?;

                todos = get_todos(&pool).await?;
                line = list_todos(&todos).await?;
            }
            Event::Key(Key::Up) => {
                print!("{}", cursor::Up(1));
                stdout.flush().unwrap();
                line -= 1;
            }
            Event::Key(Key::Down) => {
                print!("{}", cursor::Down(1));
                stdout.flush().unwrap();
                line += 1;
            }
            Event::Key(Key::Left) => {
                print!("{}", cursor::Left(1));
                stdout.flush().unwrap();
            }
            Event::Key(Key::Right) => {
                print!("{}", cursor::Right(1));
                stdout.flush().unwrap();
            }
            Event::Key(Key::Char('a')) => {
                add_view(&pool, stdout).await?;
                todos = get_todos(&pool).await?;
                line = list_todos(&todos).await?;
            }
            Event::Key(Key::Ctrl('c')) => break,
            _ => (),
        }
    }

    Ok(())
}

async fn add_view<W: Write>(pool: &SqlitePool, stdout: &mut W) -> Result<(), sqlx::Error> {
    let stdin = stdin();
    print!("{}{}New Entry : ", cursor::Goto(1, 1), clear::All,);
    stdout.flush().unwrap();

    let mut inp = String::new();
    for c in stdin.events() {
        match c.unwrap() {
            Event::Key(Key::Char('\n')) => {
                print!("{}", inp);
                stdout.flush().unwrap();
                break;
            }
            Event::Key(Key::Char(ch)) => {
                inp.push(ch);
                print!("{}", ch);
                stdout.flush().unwrap();
            }
            Event::Key(Key::Backspace) => {
                if inp.len() > 0 {
                    print!("{}{}", cursor::Left(1u16), clear::AfterCursor);
                    stdout.flush().unwrap();
                }
                inp.pop();
            }
            Event::Key(Key::Ctrl('c')) => return Ok(()),
            _ => (),
        }
    }

    match add_todo(pool, inp.clone()).await {
        Ok(_) => {
            println!(
                "{}{}Successfully added TODO : {}",
                clear::All,
                cursor::Goto(1, 1),
                inp
            );
            sleep(Duration::from_secs(1)).await;
        }
        Err(_) => {
            println!("Couldn't add todo : Something went wrong!");
            sleep(Duration::from_secs(1)).await;
        }
    }

    Ok(())
}

struct Command<'a> {
    key: &'a str,
    description: &'a str,
}

impl fmt::Display for Command<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}Press {}{}{} => {}{}",
            style::Bold,
            color::Fg(color::Green),
            self.key,
            color::Fg(color::Reset),
            self.description,
            style::Reset
        )
    }
}

async fn command_view() -> () {
    println!("{}{}Commands: ", clear::All, cursor::Goto(1, 1));
    println!("{}", cursor::Goto(1, 1));
    let mut line = 1u16;

    for cmd in [
        Command {
            key: "'a'",
            description: "Add TODO",
        },
        Command {
            key: "Enter",
            description: "Mark TODO as done",
        },
        Command {
            key: "CTRL+'C'",
            description: "Exit program",
        },
    ] {
        println!("{}{}", cursor::Goto(1, line), cmd);
        line += 1;
    }
}

enum KnownMigration {
    InitTable,
    AddHashColumn,
}

impl KnownMigration {
    fn hash(&self) -> &'static str {
        match self {
            Self::InitTable => "Init Todo Table",
            Self::AddHashColumn => "Add Hash Column to Todo Table",
        }
    }

    fn all() -> Vec<KnownMigration> {
        vec![KnownMigration::InitTable, KnownMigration::AddHashColumn]
    }
}

async fn execute(m: &KnownMigration, pool: &SqlitePool) -> Result<(), sqlx::Error> {
    match m {
        KnownMigration::InitTable => {
            sqlx::query(
                "
                CREATE TABLE IF NOT EXISTS Todo(
                    id INTEGER PRIMARY KEY,
                    content TEXT
                );
                ",
            )
            .execute(pool)
            .await?;
        }
        KnownMigration::AddHashColumn => {
            sqlx::query(
                "
                ALTER TABLE Todo
                ADD active INTEGER DEFAULT 1 CHECK(active=1 OR active=0)
                ",
            )
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

#[derive(FromRow, Debug)]
#[allow(dead_code)]
struct Migration {
    hash: String,
}

async fn perform_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS Migration(
            id INTEGER PRIMARY KEY,
            hash TEXT UNIQUE
        );
        ",
    )
    .execute(pool)
    .await?;

    let past_migrations = sqlx::query_as::<_, Migration>("SELECT hash FROM Migration;")
        .fetch_all(pool)
        .await?;

    for m in &KnownMigration::all()[past_migrations.len()..] {
        execute(&m, pool).await?;
        sqlx::query("INSERT INTO Migration (hash) VALUES (?);")
            .bind(m.hash())
            .execute(pool)
            .await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = SqlitePool::connect("sqlite://db.sqlite3?mode=rwc").await?;
    let mut stdout = stdout().into_raw_mode().unwrap();

    perform_migrations(&pool).await?;

    command_view().await;
    sleep(Duration::from_secs(2)).await;

    list_view(&pool, &mut stdout).await?;

    Ok(())
}
