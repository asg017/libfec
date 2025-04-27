use fec_api::Api;
use std::error::Error;

pub fn cmd_interactive() -> Result<(), Box<dyn Error>> {
    let client = Api::new("DEMO_KEY");
    println!("yo");
    for x in client.search_candidates("trump").unwrap() {
        println!("{:?}", x)
    }
    println!("done");
    Ok(())
}
