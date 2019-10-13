#[doc(hidden)]
#[macro_export]
macro_rules! upnp_action {
    ( $self:expr, $service:ident:$version:literal/$action:ident, $args:expr ) => {
        $self
            .0
            .find_service(&URN::service(
                "schemas-upnp-org",
                stringify!($service),
                $version,
            ))
            .expect(concat!(
                "sonos device doesn't have a ",
                stringify!($service),
                ':',
                $version,
                " service"
            ))
            .action($self.0.url(), stringify!($action), $args)
            .await
    };
}

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
    fn extract(&mut self, key: &str) -> Result<String, upnp::Error>;
}
impl HashMapExt for std::collections::HashMap<String, String> {
    fn extract(&mut self, key: &str) -> Result<String, upnp::Error> {
        self.remove(key).ok_or_else(|| upnp::Error::XMLMissingElement(
            "UPnP Response".to_string(),
            key.to_string(),
        ))
    }
}

pub fn duration_to_str(duration: &std::time::Duration) -> String {
    let seconds_total = duration.as_secs();
    let seconds = seconds_total % 60;
    let minutes = (seconds_total / 60) % 60;
    let hours = seconds_total / 3600;

    return format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
}
pub fn duration_from_str(s: &str) -> Result<std::time::Duration, upnp::Error> {
    let opt = (|| {
        let mut split = s.splitn(3, ':');
        let hours = split.next()?.parse::<u64>().ok()?;
        let minutes = split.next()?.parse::<u64>().ok()?;
        let seconds = split.next()?.parse::<u64>().ok()?;

        Some(std::time::Duration::from_secs(
            hours * 3600 + minutes * 60 + seconds,
        ))
    })();

    opt.ok_or(upnp::Error::ParseError("invalid duration"))
}

pub fn parse_bool(s: String) -> Result<bool, upnp::Error> {
    s.parse()
        .map_err(upnp::Error::invalid_response)
}

use roxmltree::{Document, Node};

pub fn parse_node_text(node: Node) -> Result<String, upnp::Error> {
    node.text()
        .ok_or_else(|| upnp::Error::XMLMissingText(node.tag_name().name().to_string()))
        .map(|x| x.to_string())
}

pub fn find_root_node<'a, 'input: 'a>(
    document: &'input Document,
    element: &str,
    docname: &str,
) -> Result<Node<'a, 'input>, upnp::Error> {
    document
        .descendants()
        .filter(roxmltree::Node::is_element)
        .find(|n| n.tag_name().name().eq_ignore_ascii_case(element))
        .ok_or_else(|| upnp::Error::XMLMissingElement(docname.to_string(), element.to_string()))
}
