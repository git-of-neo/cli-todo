use event::{Event, Key};
use sqlx::{self, FromRow, SqlitePool};
use std::fmt;
use std::io::{stdin, stdout, Write};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, cursor, event};
use tokio::time::{sleep, Duration};

#[derive(FromRow)]
struct Todo {
    content: String,
}

impl fmt::Display for Todo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Todo : {}", self.content)
    }
}

async fn get_todos(pool: &SqlitePool) -> Result<Vec<Todo>, sqlx::Error> {
    sqlx::query_as::<_, Todo>("SELECT content FROM Todo")
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

async fn list_view(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    println!("{}{}Welcome!", clear::All, cursor::Goto(1, 1));
    println!("{}", cursor::Goto(1, 1));
    let mut line = 2u16;
    let todos = get_todos(&pool).await?;
    for t in todos {
        println!("{}{}", t, cursor::Goto(1, line));
        line += 1;
    }
    Ok(())
}

async fn add_view(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut stdout = stdout().into_raw_mode().unwrap();
    let stdin = stdin();
    print!("{}{}New Entry:", cursor::Goto(1, 1), clear::All,);
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

async fn command_view() -> () {
    println!("{}{}Commands: ", clear::All, cursor::Goto(1, 1));
    println!("{}", cursor::Goto(1, 1));
    let mut line = 1u16;

    for cmd in vec!["Press 'a' => Add TODO", "Press CTRL+'C' => Exit program"] {
        println!("{}.{}{}", line, cursor::Goto(1, line), cmd);
        line += 1;
    }
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = SqlitePool::connect("sqlite://db.sqlite3?mode=rwc").await?;
    let mut stdout = stdout().into_raw_mode().unwrap();

    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS Todo(
            id INTEGER PRIMARY KEY,
            content TEXT
        );
        ",
    )
    .execute(&pool)
    .await?;

    command_view().await;
    sleep(Duration::from_secs(2)).await;

    list_view(&pool).await?;

    let stdin = stdin();
    for c in stdin.events() {
        match c.unwrap() {
            Event::Key(Key::Char('\n')) => {
                todo!("Mark todo as done")
            }
            Event::Key(Key::Up) => {
                print!("{}", cursor::Up(1));
                stdout.flush().unwrap();
            }
            Event::Key(Key::Down) => {
                print!("{}", cursor::Down(1));
                stdout.flush().unwrap();
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
                add_view(&pool).await?;
                list_view(&pool).await?
            }
            Event::Key(Key::Ctrl('c')) => break,
            _ => (),
        }
    }

    Ok(())
}
