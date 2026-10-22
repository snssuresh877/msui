use std::env;
use std::path::PathBuf;

fn main() -> iced::Result {
    let args: Vec<String> = env::args().collect();
    let initial_path = args.get(1).map(PathBuf::from);

    msui_fm::run_fm(initial_path)
}
