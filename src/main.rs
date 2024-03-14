use clap::Parser;
use editor::Editor;
use std::{fs::read_to_string, io::Write, path::PathBuf};
use terminal::Terminal;

mod editor;
mod terminal;

#[derive(Parser)]
struct Args {
    filename: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let stdout = std::io::stdout();
    let terminal = Terminal::new(stdout)?;

    let mut editor = match args.filename {
        Some(filename) => {
            let text = read_to_string(filename)?;

            Editor::new_with_text(terminal, text)
        }
        None => Editor::new(terminal),
    };

    let result = run(&mut editor);

    editor.finish()?;

    result
}

fn run(editor: &mut Editor) -> Result<(), Box<dyn std::error::Error>> {
    editor.start()?;
    editor.clear()?;
    editor.draw()?;
    editor.flush()?;

    loop {
        if let Some(result) = editor.handle_event()? {
            match result {
                editor::EventResult::Quit => break,
            }
        }

        editor.draw()?;
        editor.flush()?;
    }

    Ok(editor.finish()?)
}
