mod confer;

use confer::Confer;

fn main() {
    let confer = Confer::new();

    confer.create("config/database/host", "localhost".to_string());
    confer.create("config/database/port", "8080".to_string());
    confer.create("config/services/user/preferences/theme", "dark".to_string());
    confer.create("config/services/user/lang", "en".to_string());

    if let Some(port) = confer.get("config/database/port") {
        println!("Database Port = {}", port);
    }

    let children = confer.list_children("config/services");
    println!("Children of 'Config/services': {:?}", children);

    confer.delete("config/services/user/lang");
    if confer.get("config/services/user/lang").is_none() {
        println!("'lang' deleted successfully");
    }
}

