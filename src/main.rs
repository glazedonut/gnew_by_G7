use gnew::repo::command;
use gnew::wd::parser::{self, Gnew};
use gnew::wd::ui;

fn main() {
    let opt = parser::parse();
    println!("{:#?}", opt);
    match opt {
        Gnew::Init => todo!(),
        _ => todo!(),
    }
}
