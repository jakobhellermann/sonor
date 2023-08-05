use crate::Result;
use roxmltree::{Document, Node};

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
            rupnp::Error::XmlMissingElement("UPnP Response".to_string(), key.to_string()).into()
        })
    }
}

pub fn seconds_to_str(seconds_total: i64) -> String {
    let sign = if seconds_total < 0 { "-" } else { "" };
    let seconds_total = seconds_total.abs();

    let seconds = seconds_total % 60;
    let minutes = (seconds_total / 60) % 60;
    let hours = seconds_total / 3600;

    format!("{}{:02}:{:02}:{:02}", sign, hours, minutes, seconds)
}
pub fn seconds_from_str(s: &str) -> Result<u32> {
    let opt = (|| {
        let mut split = s.splitn(3, ':');
        let hours = split.next()?.parse::<u32>().ok()?;
        let minutes = split.next()?.parse::<u32>().ok()?;
        let seconds = split.next()?.parse::<u32>().ok()?;

        Some(hours * 3600 + minutes * 60 + seconds)
    })();

    opt.ok_or(rupnp::Error::ParseError("invalid duration").into())
}

pub fn parse_bool(s: String) -> Result<bool> {
    match s.trim() {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(rupnp::Error::ParseError("bool was neither `0` nor `1`").into()),
    }
}

pub fn find_node_attribute<'n, 'd: 'n>(node: Node<'d, 'n>, attr: &str) -> Result<&'n str> {
    node.attributes()
        .find(|a| a.name().eq_ignore_ascii_case(attr))
        .map(|attr| attr.value())
        .ok_or_else(|| {
            rupnp::Error::XmlMissingElement(node.tag_name().name().to_string(), attr.to_string())
                .into()
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
        .ok_or_else(|| {
            rupnp::Error::XmlMissingElement(docname.to_string(), element.to_string()).into()
        })
}
