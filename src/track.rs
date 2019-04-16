use getset::Getters;
use std::time::Duration;
use xmltree::Element;

const SECS_PER_MINUTE: u64 = 60;
const MINS_PER_HOUR: u64 = 60;
const SECS_PER_HOUR: u64 = 3600;

pub(crate) fn duration_from_str(s: &str) -> Option<Duration> {
    let mut split = s.splitn(3, ':');
    let hours = split.next()?.parse::<u64>().ok()?;
    let minutes = split.next()?.parse::<u64>().ok()?;
    let seconds = split.next()?.parse::<u64>().ok()?;

    Some(Duration::from_secs(
        hours * SECS_PER_HOUR + minutes * SECS_PER_MINUTE + seconds,
    ))
}
pub(crate) fn duration_to_str(duration: &Duration) -> String {
    let seconds_total = duration.as_secs();

    let seconds = seconds_total % SECS_PER_MINUTE;
    let minutes = (seconds_total / SECS_PER_MINUTE) % MINS_PER_HOUR;
    let hours = seconds_total / SECS_PER_HOUR;

    return format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
}

#[derive(Debug)]
pub struct TrackInfo {
    track: Track,
    track_no: u32,
    duration: Duration,
    elapsed: Duration,
}
impl TrackInfo {
    pub fn new(track: Track, track_no: u32, duration: Duration, played: Duration) -> Self {
        Self {
            track,
            track_no,
            duration,
            elapsed: played,
        }
    }

    pub fn track(&self) -> &Track {
        &self.track
    }
    pub fn track_no(&self) -> u32 {
        self.track_no
    }
    pub fn duration(&self) -> &Duration {
        &self.duration
    }
    pub fn elapsed(&self) -> &Duration {
        &self.elapsed
    }
}

#[derive(Debug, Getters)]
#[get = "pub"]
pub struct Track {
    title: String,
    creator: String,
    album: Option<String>,
    duration: Duration,
    res: String,
}

impl std::fmt::Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} - {}", &self.title, &self.creator)?;
        if let Some(album) = &self.album {
            write!(f, " ({})", album)?;
        }
        Ok(())
    }
}

fn text_or_none(elem: Option<Element>) -> Option<String> {
    match elem {
        Some(x) => Some(x.text?),
        None => None,
    }
}

impl Track {
    pub(crate) fn from_xml(mut item: Element) -> Option<Self> {
        let title = item.take_child("title")?.text?;
        let creator = item.take_child("creator")?.text?;
        let album = text_or_none(item.take_child("album"));
        let (res, duration) = {
            let mut res = item.take_child("res")?;
            let duration = res.attributes.remove("duration")?;

            (res.text?, duration_from_str(&duration)?)
        };

        Some(Track {
            title,
            creator,
            album,
            res,
            duration,
        })
    }
}
