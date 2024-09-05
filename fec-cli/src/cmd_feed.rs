use indicatif::HumanDuration;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::time::Instant;

#[derive(Debug)]
struct RssFeedItem {
    title: String,
    link: String,
    description: String,
    guid: String,
    date: String,
}

fn item_read(reader: &mut Reader<&[u8]>) -> Result<RssFeedItem, ()> {
    let mut buf = Vec::new();
    let mut last_key = String::new();
    let mut last_text = String::new();

    let mut title = None;
    let mut link = None;
    let mut description = None;
    let mut pub_date = None;
    let mut guid = None;
    let mut dc_date = None;
    loop {
        // buffer, we could directly call `reader.read_event()`
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            Ok(Event::Eof) => todo!(),
            Ok(Event::Start(e)) => {}
            Ok(Event::Text(e)) => last_text = std::str::from_utf8(e.as_ref()).unwrap().to_string(),
            Ok(Event::End(e)) => {
                match e.as_ref() {
                    b"item" => break,
                    b"title" => title = Some(last_text.clone()),
                    b"link" => link = Some(last_text.clone()),
                    b"description" => description = Some(last_text.clone()),
                    b"pubDate" => pub_date = Some(last_text.clone()),
                    b"guid" => guid = Some(last_text.clone()),
                    b"dc:date" => dc_date = Some(last_text.clone()),
                    key => todo!("{}", std::str::from_utf8(key.as_ref()).unwrap()),
                }
                if e.as_ref() == b"item" {
                    break;
                }
            }
            _ => panic!(),
        }
    }
    Ok(RssFeedItem {
        title: title.ok_or_else(|| ())?,
        link: link.ok_or_else(|| ())?,
        description: description.ok_or_else(|| ())?,
        guid: guid.ok_or_else(|| ())?,
        date: dc_date.ok_or_else(|| ())?,
    })
}

pub fn cmd_feed() -> Result<(), ()> {
    let t0 = Instant::now();

    let request = ureq::get("https://efilingapps.fec.gov/rss/generate?preDefinedFilingType=ALL");
    let response = request.call().unwrap();
    let contents = response.into_string().unwrap();
    let mut items = vec![];

    let mut reader = Reader::from_str(&contents);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        // buffer, we could directly call `reader.read_event()`
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => {
                println!("Start: {:?}", e.as_ref());
                if e.as_ref() == b"item" {
                    let item = item_read(&mut reader).unwrap();
                    items.push(item);
                }
            }
            Ok(Event::End(e)) => {}
            Ok(Event::Text(e)) => {}

            // There are several other `Event`s we do not consider here
            _ => (),
        }
        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }

    println!("Finished in {}", HumanDuration(Instant::now() - t0));

    /*let mut child = Command::new("vd")
        .arg("tmp.zip")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit()) // inherit stderr to see any errors
        .spawn()
        .expect("Failed to spawn subprocess");
    child.wait().unwrap();*/
    Ok(())
}
