extern crate pretty_env_logger;
#[macro_use] extern crate log;

mod app;

use crate::app::App;
use crate::app::block::Block;

fn main() {

    pretty_env_logger::init();

    let mut app = App::new();

    app.genesis();
    app.print_chain();
    assert!(app.blocks.len() == 1);

    let prev_block = &app.blocks[0];
    let new_id = prev_block.id + 1;
    let new_data = format!("Block #{}", new_id);
    let new_block = Block::new(new_id, prev_block.hash.to_string(), new_data);

    app.try_add_block(new_block);
    app.print_chain();
    assert!(app.blocks.len() == 2);
}
