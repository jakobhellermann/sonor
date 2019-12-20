use crate::Result;
use roxmltree::{Attribute, Document, Node};

#[doc(hidden)]
#[macro_export]
macro_rules! args {
    ( $( $var:literal: $e:expr ),* ) => { &{
        let mut s = String::new();
        $(
            s.push_str(concat!("<", $var, ">"));
            s.push_str(&$e.to_string());
            s.push_str(concat!("</", $var, ">"));
        )*
        s
    } }
}

pub trait HashMapExt {
    fn extract(&mut self, key: &str) -> Result<String>;
}
impl HashMapExt for std::collections::HashMap<String, String> {
    fn extract(&mut self, key: &str) -> Result<String> {
        self.remove(key).ok_or_else(|| {
            upnp::Error::XmlMissingElement("UPnP Response".to_string(), key.to_string())
        })
    }
}

pub fn seconds_to_str(seconds_total: u32) -> String {
    let seconds = seconds_total % 60;
    let minutes = (seconds_total / 60) % 60;
    let hours = seconds_total / 3600;

    return format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
}
pub fn seconds_from_str(s: &str) -> Result<u32> {
    let opt = (|| {
        let mut split = s.splitn(3, ':');
        let hours = split.next()?.parse::<u32>().ok()?;
        let minutes = split.next()?.parse::<u32>().ok()?;
        let seconds = split.next()?.parse::<u32>().ok()?;

        Some(hours * 3600 + minutes * 60 + seconds)
    })();

    opt.ok_or(upnp::Error::ParseError("invalid duration"))
}

pub fn parse_bool(s: String) -> Result<bool> {
    match s.trim() {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(upnp::Error::ParseError("bool was neither `0` nor `1`")),
    }
}

pub fn find_node_attribute<'n, 'd: 'n>(node: Node<'d, 'n>, attr: &str) -> Result<&'n str> {
    node.attributes()
        .iter()
        .find(|a| a.name().eq_ignore_ascii_case(attr))
        .map(Attribute::value)
        .ok_or_else(|| {
            upnp::Error::XmlMissingElement(node.tag_name().name().to_string(), attr.to_string())
        })
}

pub fn find_root_node<'a, 'input: 'a>(
    document: &'input Document<'input>,
    element: &str,
    docname: &str,
) -> Result<Node<'a, 'input>> {
    document
        .descendants()
        .filter(roxmltree::Node::is_element)
        .find(|n| n.tag_name().name().eq_ignore_ascii_case(element))
        .ok_or_else(|| upnp::Error::XmlMissingElement(docname.to_string(), element.to_string()))
}
