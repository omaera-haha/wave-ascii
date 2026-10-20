//! SVG animation (SMIL): `<animate>`, `<set>`, `<animateTransform>`, and
//! `<animateMotion>`, which browsers play but usvg draws as a still picture.
//! Each moment becomes a still SVG by writing the animated attribute values
//! into the source text.

use std::ops::Range;
use std::str::FromStr;

use glam::Vec2;
use resvg::usvg::roxmltree::{self, Document, Node, ParsingOptions};

const SVG: &str = "http://www.w3.org/2000/svg";
const XLINK: &str = "http://www.w3.org/1999/xlink";

/// An SVG document and the animations in it.
pub struct Animation {
    source: String,
    /// In source order.
    targets: Vec<Target>,
}

impl Animation {
    /// Finds the animations in an SVG document, or `None` if it has none
    /// that start on their own. Animations that begin on an event, such as a
    /// click or another animation ending, never start.
    pub fn parse(source: &str) -> Result<Option<Self>, roxmltree::Error> {
        let options = ParsingOptions {
            allow_dtd: true,
            ..ParsingOptions::default()
        };
        let document = Document::parse_with_options(source, options)?;
        let mut targets: Vec<Target> = Vec::new();
        for node in document.descendants() {
            let Some(kind) = Kind::of(node) else {
                continue;
            };
            let Some(element) = target_element(node, &document) else {
                continue;
            };
            let name = match kind {
                Kind::Motion => "transform",
                Kind::Transform => node.attribute("attributeName").unwrap_or("transform"),
                Kind::Attribute | Kind::Set => match node.attribute("attributeName") {
                    Some(name) => name,
                    None => continue,
                },
            };
            let local_name = name.rsplit(':').next().unwrap_or(name);
            let existing = element
                .attributes()
                .find(|attribute| attribute.name() == local_name);
            let base = existing.map_or("", |attribute| attribute.value());
            let Some(animation) = Animate::read(node, kind, base, &document) else {
                continue;
            };

            let start = element.range().start;
            if let Some(target) = targets
                .iter_mut()
                .find(|target| target.element == start && target.name == name)
            {
                target.animations.push(animation);
                continue;
            }
            let place = match existing {
                Some(attribute) => Place::Value(attribute.range_value()),
                None => {
                    // Right after the tag name.
                    let name_start = start + 1;
                    Place::Missing(
                        source[name_start..]
                            .find(|character: char| {
                                character.is_whitespace() || character == '/' || character == '>'
                            })
                            .map_or(source.len(), |offset| name_start + offset),
                    )
                }
            };
            targets.push(Target {
                element: start,
                place,
                name: name.to_owned(),
                base: base.to_owned(),
                animations: vec![animation],
            });
        }
        for target in &mut targets {
            // Animations that begin later take priority.
            target
                .animations
                .sort_by(|a, b| a.timing.begin.total_cmp(&b.timing.begin));
        }
        targets.sort_by_key(|target| target.place.start());
        Ok((!targets.is_empty()).then(|| Self {
            source: source.to_owned(),
            targets,
        }))
    }

    /// The still SVG at `time` seconds.
    pub fn at(&self, time: f32) -> String {
        let mut svg = String::with_capacity(self.source.len() + 64 * self.targets.len());
        let mut copied = 0;
        for target in &self.targets {
            let Some(value) = target.value(time) else {
                continue;
            };
            let value = escape(&value);
            match &target.place {
                Place::Value(range) => {
                    svg += &self.source[copied..range.start];
                    svg += &value;
                    copied = range.end;
                }
                Place::Missing(position) => {
                    svg += &self.source[copied..*position];
                    svg += &format!(" {}=\"{value}\"", target.name);
                    copied = *position;
                }
            }
        }
        svg += &self.source[copied..];
        svg
    }
}

/// One attribute of one element and the animations that change it.
struct Target {
    /// Where the element starts in the source.
    element: usize,
    place: Place,
    name: String,
    /// The attribute's value when no animation changes it.
    base: String,
    animations: Vec<Animate>,
}

impl Target {
    /// The attribute's value at `time`, or `None` while no animation is
    /// changing it.
    fn value(&self, time: f32) -> Option<String> {
        let mut value = None;
        let mut motion = None;
        for animation in &self.animations {
            let Some(progress) = animation.timing.progress(time) else {
                continue;
            };
            let sample = animation.sample(progress);
            if let Effect::Motion { .. } = animation.effect {
                motion = Some(sample);
                continue;
            }
            let underlying = value.as_deref().unwrap_or(self.base.as_str());
            value = Some(match (&animation.effect, animation.additive) {
                (Effect::Transform { .. }, true) if !underlying.trim().is_empty() => {
                    format!("{underlying} {sample}")
                }
                (Effect::Attribute(_), true) => Value::parse(underlying)
                    .add(&Value::parse(&sample))
                    .to_string(),
                _ => sample,
            });
        }
        // Motion moves the element in its parent's space, outside its own
        // transform.
        match (motion, value) {
            (Some(motion), value) => {
                let transform = value.as_deref().unwrap_or(self.base.as_str()).trim();
                Some(if transform.is_empty() {
                    motion
                } else {
                    format!("{motion} {transform}")
                })
            }
            (None, value) => value,
        }
    }
}

enum Place {
    /// The range of the attribute's value in the source.
    Value(Range<usize>),
    /// Where to add the attribute, since the element doesn't have it.
    Missing(usize),
}

impl Place {
    fn start(&self) -> usize {
        match self {
            Self::Value(range) => range.start,
            Self::Missing(position) => *position,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Attribute,
    Set,
    Transform,
    Motion,
}

impl Kind {
    fn of(node: Node) -> Option<Self> {
        let name = node.tag_name();
        if name.namespace() != Some(SVG) {
            return None;
        }
        match name.name() {
            "animate" | "animateColor" => Some(Self::Attribute),
            "set" => Some(Self::Set),
            "animateTransform" => Some(Self::Transform),
            "animateMotion" => Some(Self::Motion),
            _ => None,
        }
    }
}

/// The element an `href` names, or else the animation's parent.
fn target_element<'a, 'input>(
    node: Node<'a, 'input>,
    document: &'a Document<'input>,
) -> Option<Node<'a, 'input>> {
    match href(node) {
        Some(reference) => element_by_id(document, reference),
        None => node.parent_element(),
    }
}

fn href<'a>(node: Node<'a, '_>) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| {
            attribute.name() == "href" && matches!(attribute.namespace(), None | Some(XLINK))
        })
        .map(|attribute| attribute.value())
}

fn element_by_id<'a, 'input>(
    document: &'a Document<'input>,
    reference: &str,
) -> Option<Node<'a, 'input>> {
    let id = reference.trim().strip_prefix('#')?;
    document
        .descendants()
        .find(|node| node.attribute("id") == Some(id))
}

/// One animation element.
struct Animate {
    timing: Timing,
    keyframes: Keyframes,
    effect: Effect,
    /// Whether it adds to the value beneath it rather than replacing it.
    additive: bool,
}

enum Effect {
    Attribute(Vec<Value>),
    /// A transform of one type, such as `rotate`, with its parameters at
    /// each keyframe.
    Transform {
        kind: String,
        values: Vec<Vec<f32>>,
    },
    /// Movement along `track` to the fraction of its length in `points` at
    /// each keyframe.
    Motion {
        track: Track,
        rotate: Rotate,
        points: Vec<f32>,
    },
}

impl Animate {
    fn read(node: Node, kind: Kind, base: &str, document: &Document) -> Option<Self> {
        let timing = Timing::read(node)?;
        let list = |name| {
            node.attribute(name).map(|values: &str| {
                values
                    .split(';')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
            })
        };
        // Motion defaults to an even speed along its path.
        let mut mode = match (kind, node.attribute("calcMode")) {
            (Kind::Motion, None) => Mode::Paced,
            _ => Mode::read(node),
        };
        let (effect, count) = match kind {
            Kind::Set => {
                mode = Mode::Discrete;
                (
                    Effect::Attribute(vec![Value::parse(node.attribute("to")?)]),
                    1,
                )
            }
            Kind::Attribute => {
                let values = match list("values") {
                    Some(values) => values.into_iter().map(Value::parse).collect(),
                    None => {
                        let from = Value::parse(node.attribute("from").unwrap_or(base));
                        let to = match (node.attribute("to"), node.attribute("by")) {
                            (Some(to), _) => Value::parse(to),
                            (None, Some(by)) => from.add(&Value::parse(by)),
                            (None, None) => return None,
                        };
                        vec![from, to]
                    }
                };
                let count = values.len();
                (Effect::Attribute(values), count)
            }
            Kind::Transform => {
                let kind = node.attribute("type").unwrap_or("translate");
                if !["translate", "scale", "rotate", "skewX", "skewY"].contains(&kind) {
                    return None;
                }
                let parameters = |text: &str| numbers(text).map(|list| pad(kind, list));
                let values = match list("values") {
                    Some(values) => values
                        .into_iter()
                        .map(parameters)
                        .collect::<Option<Vec<_>>>()?,
                    None => {
                        let to = node.attribute("to").and_then(parameters);
                        let by = node.attribute("by").and_then(parameters);
                        let from = match node.attribute("from") {
                            Some(from) => parameters(from)?,
                            None => identity(kind, to.as_ref().or(by.as_ref())?),
                        };
                        let to = match (to, by) {
                            (Some(to), _) => to,
                            (None, Some(by)) if by.len() == from.len() => {
                                from.iter().zip(&by).map(|(a, b)| a + b).collect()
                            }
                            _ => return None,
                        };
                        vec![from, to]
                    }
                };
                let count = values.len();
                (
                    Effect::Transform {
                        kind: kind.to_owned(),
                        values,
                    },
                    count,
                )
            }
            Kind::Motion => {
                let path = node
                    .children()
                    .find(|child| child.tag_name().name() == "mpath")
                    .and_then(href)
                    .and_then(|reference| element_by_id(document, reference))
                    .and_then(|path| path.attribute("d"))
                    .or_else(|| node.attribute("path"));
                let (track, corners) = match path {
                    Some(data) => (Track::parse(data)?, None),
                    None => {
                        let point = |text: &str| match numbers(text)?.as_slice() {
                            &[x, y] => Some(Vec2::new(x, y)),
                            _ => None,
                        };
                        let points = match list("values") {
                            Some(values) => {
                                values.into_iter().map(point).collect::<Option<Vec<_>>>()?
                            }
                            None => {
                                let to = node.attribute("to").and_then(point);
                                let by = node.attribute("by").and_then(point);
                                let from = node.attribute("from").and_then(point);
                                let from = from.unwrap_or(Vec2::ZERO);
                                vec![from, to.or(by.map(|by| from + by))?]
                            }
                        };
                        let track = Track::through(&points);
                        let corners = track.corners();
                        (track, Some(corners))
                    }
                };
                let rotate = Rotate::read(node.attribute("rotate"));
                let key_points = node.attribute("keyPoints").and_then(numbers);
                let points = match (mode, key_points, corners) {
                    (Mode::Paced, ..) => vec![0.0, 1.0],
                    (_, Some(points), _) => points,
                    (_, None, Some(corners)) => corners,
                    (_, None, None) => vec![0.0, 1.0],
                };
                let count = points.len();
                (
                    Effect::Motion {
                        track,
                        rotate,
                        points,
                    },
                    count,
                )
            }
        };
        if count == 0 {
            return None;
        }
        // Paced motion has its even speed from `points`; other paced
        // animations are treated as linear.
        if mode == Mode::Paced {
            mode = Mode::Linear;
        }
        Some(Self {
            timing,
            keyframes: Keyframes::read(node, mode, count),
            effect,
            additive: kind != Kind::Set && node.attribute("additive") == Some("sum"),
        })
    }

    /// The animation's value `progress` of the way through its simple
    /// duration.
    fn sample(&self, progress: f32) -> String {
        let (from, to, fraction) = self.keyframes.locate(progress);
        match &self.effect {
            Effect::Attribute(values) => values[from].mix(&values[to], fraction).to_string(),
            Effect::Transform { kind, values } => {
                let (a, b) = (&values[from], &values[to]);
                let parameters: Vec<String> = if a.len() == b.len() {
                    a.iter()
                        .zip(b)
                        .map(|(a, b)| (a + (b - a) * fraction).to_string())
                        .collect()
                } else {
                    let nearest = if fraction < 1.0 { a } else { b };
                    nearest.iter().map(f32::to_string).collect()
                };
                format!("{kind}({})", parameters.join(" "))
            }
            Effect::Motion {
                track,
                rotate,
                points,
            } => {
                let along = points[from] + (points[to] - points[from]) * fraction;
                let (position, direction) = track.at(along);
                let angle = match rotate {
                    Rotate::Fixed(angle) => *angle,
                    Rotate::Auto(offset) => direction.y.atan2(direction.x).to_degrees() + offset,
                };
                if angle == 0.0 {
                    format!("translate({} {})", position.x, position.y)
                } else {
                    format!("translate({} {}) rotate({angle})", position.x, position.y)
                }
            }
        }
    }
}

/// When an animation plays, in seconds.
struct Timing {
    begin: f32,
    /// The simple duration, which may be infinite.
    duration: f32,
    /// How long it plays, counting repeats.
    active: f32,
    /// Whether the last value holds after it ends.
    freeze: bool,
}

impl Timing {
    fn read(node: Node) -> Option<Self> {
        let first_offset = |list: &str| list.split(';').find_map(clock);
        let begin = match node.attribute("begin") {
            Some(list) => first_offset(list)?,
            None => 0.0,
        };
        let indefinite = |text: &str| text.trim() == "indefinite";
        let duration = node
            .attribute("dur")
            .and_then(clock)
            .filter(|duration| *duration > 0.0)
            .unwrap_or(f32::INFINITY);
        let repeats = node.attribute("repeatCount").and_then(|count| {
            if indefinite(count) {
                Some(f32::INFINITY)
            } else {
                count.trim().parse().ok().filter(|count: &f32| *count > 0.0)
            }
        });
        let repeat_duration = node.attribute("repeatDur").and_then(|length| {
            if indefinite(length) {
                Some(f32::INFINITY)
            } else {
                clock(length)
            }
        });
        let mut active = match (repeats, repeat_duration) {
            (None, None) => duration,
            (count, length) => {
                (count.unwrap_or(f32::INFINITY) * duration).min(length.unwrap_or(f32::INFINITY))
            }
        };
        if let Some(end) = node.attribute("end").and_then(first_offset) {
            active = active.min(end - begin);
        }
        if active <= 0.0 {
            return None;
        }
        Some(Self {
            begin,
            duration,
            active,
            freeze: node.attribute("fill") == Some("freeze"),
        })
    }

    /// How far through its simple duration the animation is at `time`, from
    /// 0 to 1, or `None` when it has no effect then.
    fn progress(&self, time: f32) -> Option<f32> {
        let elapsed = time - self.begin;
        if elapsed < 0.0 || (elapsed >= self.active && !self.freeze) {
            return None;
        }
        if self.duration.is_infinite() {
            return Some(0.0);
        }
        let elapsed = elapsed.min(self.active);
        let progress = (elapsed / self.duration).fract();
        // Frozen at the end of a repeat, it keeps the last value rather than
        // the first.
        Some(
            if progress == 0.0 && elapsed > 0.0 && elapsed >= self.active {
                1.0
            } else {
                progress
            },
        )
    }
}

/// Parses a clock value such as `2s`, `150ms`, `-1.5`, or `0:01:30` into
/// seconds.
fn clock(text: &str) -> Option<f32> {
    let text = text.trim();
    let seconds = if text.contains(':') {
        let parts: Vec<&str> = text.split(':').collect();
        if parts.len() > 3 {
            return None;
        }
        parts.iter().try_fold(0.0, |total, part| {
            Some(total * 60.0 + part.trim().parse::<f32>().ok()?)
        })?
    } else {
        let (number, scale) = [("ms", 0.001), ("min", 60.0), ("h", 3600.0), ("s", 1.0)]
            .into_iter()
            .find_map(|(unit, scale)| text.strip_suffix(unit).map(|number| (number, scale)))
            .unwrap_or((text, 1.0));
        number.trim_end().parse::<f32>().ok()? * scale
    };
    seconds.is_finite().then_some(seconds)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Discrete,
    Linear,
    Paced,
    Spline,
}

impl Mode {
    fn read(node: Node) -> Self {
        match node.attribute("calcMode") {
            Some("discrete") => Self::Discrete,
            Some("paced") => Self::Paced,
            Some("spline") => Self::Spline,
            _ => Self::Linear,
        }
    }
}

/// When an animation reaches each of its values and how it moves between
/// them.
struct Keyframes {
    mode: Mode,
    /// Fractions of the simple duration, one per value.
    times: Vec<f32>,
    /// A cubic Bézier easing for each interval in spline mode.
    splines: Vec<[f32; 4]>,
}

impl Keyframes {
    fn read(node: Node, mode: Mode, count: usize) -> Self {
        let splines: Vec<[f32; 4]> = node
            .attribute("keySplines")
            .map(|list| {
                list.split(';')
                    .filter(|spline| !spline.trim().is_empty())
                    .filter_map(|spline| numbers(spline)?.try_into().ok())
                    .collect()
            })
            .unwrap_or_default();
        let mode = if mode == Mode::Spline && splines.len() + 1 != count {
            Mode::Linear
        } else {
            mode
        };
        let even = |intervals: usize| -> Vec<f32> {
            (0..count)
                .map(|index| index as f32 / intervals.max(1) as f32)
                .collect()
        };
        let times = node
            .attribute("keyTimes")
            .and_then(|list| numbers(&list.replace(';', " ")))
            .filter(|times| {
                times.len() == count
                    && times.first() == Some(&0.0)
                    && (mode == Mode::Discrete || times.last() == Some(&1.0))
                    && times.windows(2).all(|pair| pair[0] <= pair[1])
            })
            .unwrap_or_else(|| match mode {
                Mode::Discrete => even(count),
                _ => even(count - 1),
            });
        Self {
            mode,
            times,
            splines,
        }
    }

    /// The keyframes `progress` falls between and how far it is from the
    /// first to the second.
    fn locate(&self, progress: f32) -> (usize, usize, f32) {
        let last = self.times.len() - 1;
        let index = self
            .times
            .partition_point(|time| *time <= progress)
            .saturating_sub(1);
        if self.mode == Mode::Discrete || last == 0 {
            return (index, index, 0.0);
        }
        let index = index.min(last - 1);
        let span = self.times[index + 1] - self.times[index];
        let fraction = if span > 0.0 {
            ((progress - self.times[index]) / span).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let fraction = match self.mode {
            Mode::Spline => ease(self.splines[index], fraction),
            _ => fraction,
        };
        (index, index + 1, fraction)
    }
}

/// The cubic Bézier easing from (0, 0) to (1, 1) through two control points,
/// as in CSS's `cubic-bezier`.
fn ease([x1, y1, x2, y2]: [f32; 4], x: f32) -> f32 {
    let bezier = |a: f32, b: f32, t: f32| {
        let u = 1.0 - t;
        3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t
    };
    // x rises with t when the control points are within [0, 1], so bisect.
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..24 {
        let middle = 0.5 * (low + high);
        if bezier(x1, x2, middle) < x {
            low = middle;
        } else {
            high = middle;
        }
    }
    bezier(y1, y2, 0.5 * (low + high))
}

/// Numbers separated by spaces or commas.
fn numbers(text: &str) -> Option<Vec<f32>> {
    text.split(|character: char| character.is_whitespace() || character == ',')
        .filter(|number| !number.is_empty())
        .map(|number| number.parse().ok())
        .collect()
}

/// Fills in the parameters a transform leaves out, so `rotate(90)` can blend
/// with `rotate(180 50 50)`.
fn pad(kind: &str, numbers: Vec<f32>) -> Vec<f32> {
    match (kind, numbers.as_slice()) {
        ("rotate", &[angle]) => vec![angle, 0.0, 0.0],
        ("scale", &[scale]) => vec![scale, scale],
        ("translate", &[x]) => vec![x, 0.0],
        _ => numbers,
    }
}

/// The parameters that leave things where they are, shaped like `like`.
fn identity(kind: &str, like: &[f32]) -> Vec<f32> {
    match kind {
        "scale" => vec![1.0; like.len()],
        // Turning about the same center.
        "rotate" => [0.0]
            .into_iter()
            .chain(like.iter().skip(1).copied())
            .collect(),
        _ => vec![0.0; like.len()],
    }
}

/// An attribute value split into numbers, which animate, and the text around
/// them, so `M0 0 L10 0` blends with `M0 5 L10 5`. Colors become `rgba(…)`.
#[derive(Clone, Debug, PartialEq)]
struct Value {
    numbers: Vec<f32>,
    /// The text before, between, and after the numbers.
    text: Vec<String>,
}

impl Value {
    fn parse(source: &str) -> Self {
        let source = source.trim();
        if let Ok(color) = svgtypes::Color::from_str(source) {
            return Self {
                numbers: vec![
                    color.red.into(),
                    color.green.into(),
                    color.blue.into(),
                    f32::from(color.alpha) / 255.0,
                ],
                text: ["rgba(", ",", ",", ",", ")"].map(str::to_owned).into(),
            };
        }
        let mut value = Self {
            numbers: Vec::new(),
            text: Vec::new(),
        };
        let mut text_start = 0;
        let mut index = 0;
        while index < source.len() {
            match number_end(source, index) {
                Some(end) => {
                    value.text.push(source[text_start..index].to_owned());
                    value
                        .numbers
                        .push(source[index..end].parse().unwrap_or(0.0));
                    index = end;
                    text_start = end;
                }
                None => index += source[index..].chars().next().map_or(1, char::len_utf8),
            }
        }
        value.text.push(source[text_start..].to_owned());
        // Numbers inside references or functions, as in `url(#fade2)`, are
        // names rather than amounts.
        let plain = |text: &String| {
            text.chars().all(|character| {
                character.is_whitespace() || character.is_alphabetic() || ",%".contains(character)
            })
        };
        if value.text.iter().all(plain) {
            value
        } else {
            Self {
                numbers: Vec::new(),
                text: vec![source.to_owned()],
            }
        }
    }

    /// Whether the values differ only in their numbers.
    fn blends_with(&self, other: &Self) -> bool {
        let words = |text: &str| -> Vec<String> {
            text.split(|character: char| character.is_whitespace() || character == ',')
                .filter(|word| !word.is_empty())
                .map(str::to_owned)
                .collect()
        };
        self.numbers.len() == other.numbers.len()
            && self
                .text
                .iter()
                .zip(&other.text)
                .all(|(a, b)| words(a) == words(b))
    }

    /// The value `fraction` of the way to `other`, or one or the other when
    /// they don't blend.
    fn mix(&self, other: &Self, fraction: f32) -> Self {
        if !self.blends_with(other) {
            return if fraction < 1.0 { self } else { other }.clone();
        }
        Self {
            numbers: self
                .numbers
                .iter()
                .zip(&other.numbers)
                .map(|(a, b)| a + (b - a) * fraction)
                .collect(),
            text: self.text.clone(),
        }
    }

    fn add(&self, other: &Self) -> Self {
        if !self.blends_with(other) {
            return other.clone();
        }
        Self {
            numbers: self
                .numbers
                .iter()
                .zip(&other.numbers)
                .map(|(a, b)| a + b)
                .collect(),
            text: self.text.clone(),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (text, number) in self.text.iter().zip(&self.numbers) {
            write!(formatter, "{text}{number}")?;
        }
        write!(formatter, "{}", self.text.last().map_or("", String::as_str))
    }
}

/// Where the number starting at `start` ends, if one does.
fn number_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let digits = |mut index: usize| {
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        index
    };
    let mut index = start;
    if matches!(bytes.get(index), Some(b'+' | b'-')) {
        index += 1;
    }
    let whole = digits(index);
    let mut end = whole;
    if bytes.get(end) == Some(&b'.') {
        end = digits(end + 1);
    }
    // At least one digit, before or after the point.
    if end - index - usize::from(end > whole) == 0 {
        return None;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let mut exponent = end + 1;
        if matches!(bytes.get(exponent), Some(b'+' | b'-')) {
            exponent += 1;
        }
        let exponent_end = digits(exponent);
        if exponent_end > exponent {
            end = exponent_end;
        }
    }
    Some(end)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Rotate {
    /// Degrees.
    Fixed(f32),
    /// Along the direction of travel, turned by this many degrees.
    Auto(f32),
}

impl Rotate {
    fn read(text: Option<&str>) -> Self {
        match text.map(str::trim) {
            Some("auto") => Self::Auto(0.0),
            Some("auto-reverse") => Self::Auto(180.0),
            Some(angle) => Self::Fixed(angle.parse().unwrap_or(0.0)),
            None => Self::Fixed(0.0),
        }
    }
}

/// A motion path as straight segments, with the distance along it to each
/// point.
struct Track {
    points: Vec<Vec2>,
    distances: Vec<f32>,
}

impl Track {
    const CURVE_STEPS: usize = 16;

    fn parse(data: &str) -> Option<Self> {
        use svgtypes::SimplePathSegment as Segment;

        let mut track = Self {
            points: Vec::new(),
            distances: Vec::new(),
        };
        let mut start = Vec2::ZERO;
        let mut current = Vec2::ZERO;
        for segment in svgtypes::SimplifyingPathParser::from(data) {
            match segment.ok()? {
                Segment::MoveTo { x, y } => {
                    current = Vec2::new(x as f32, y as f32);
                    start = current;
                    // Jumps to a new subpath without covering distance.
                    let distance = track.distances.last().copied().unwrap_or(0.0);
                    track.points.push(current);
                    track.distances.push(distance);
                }
                Segment::LineTo { x, y } => {
                    current = Vec2::new(x as f32, y as f32);
                    track.extend(current);
                }
                Segment::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    let [p0, p1, p2, p3] = [
                        current,
                        Vec2::new(x1 as f32, y1 as f32),
                        Vec2::new(x2 as f32, y2 as f32),
                        Vec2::new(x as f32, y as f32),
                    ];
                    for step in 1..=Self::CURVE_STEPS {
                        let t = step as f32 / Self::CURVE_STEPS as f32;
                        let u = 1.0 - t;
                        track.extend(
                            p0 * (u * u * u)
                                + p1 * (3.0 * u * u * t)
                                + p2 * (3.0 * u * t * t)
                                + p3 * (t * t * t),
                        );
                    }
                    current = p3;
                }
                Segment::Quadratic { x1, y1, x, y } => {
                    let [p0, p1, p2] = [
                        current,
                        Vec2::new(x1 as f32, y1 as f32),
                        Vec2::new(x as f32, y as f32),
                    ];
                    for step in 1..=Self::CURVE_STEPS {
                        let t = step as f32 / Self::CURVE_STEPS as f32;
                        let u = 1.0 - t;
                        track.extend(p0 * (u * u) + p1 * (2.0 * u * t) + p2 * (t * t));
                    }
                    current = p2;
                }
                Segment::ClosePath => {
                    track.extend(start);
                    current = start;
                }
            }
        }
        (!track.points.is_empty()).then_some(track)
    }

    fn through(points: &[Vec2]) -> Self {
        let mut track = Self {
            points: Vec::new(),
            distances: Vec::new(),
        };
        for point in points {
            track.extend(*point);
        }
        track
    }

    fn extend(&mut self, point: Vec2) {
        let distance = match self.points.last() {
            Some(last) => self.distances.last().copied().unwrap_or(0.0) + last.distance(point),
            None => 0.0,
        };
        self.points.push(point);
        self.distances.push(distance);
    }

    /// How far along the track each point is, as a fraction of its length.
    fn corners(&self) -> Vec<f32> {
        let length = self.distances.last().copied().unwrap_or(0.0);
        self.distances
            .iter()
            .map(|distance| if length > 0.0 { distance / length } else { 0.0 })
            .collect()
    }

    /// The point `fraction` of the way along and the direction of travel
    /// there.
    fn at(&self, fraction: f32) -> (Vec2, Vec2) {
        let length = self.distances.last().copied().unwrap_or(0.0);
        if self.points.len() < 2 {
            return (self.points.first().copied().unwrap_or(Vec2::ZERO), Vec2::X);
        }
        let distance = fraction.clamp(0.0, 1.0) * length;
        let end = self
            .distances
            .partition_point(|reached| *reached < distance)
            .clamp(1, self.points.len() - 1);
        let (a, b) = (self.points[end - 1], self.points[end]);
        let span = self.distances[end] - self.distances[end - 1];
        let t = if span > 0.0 {
            (distance - self.distances[end - 1]) / span
        } else {
            0.0
        };
        (a.lerp(b, t), (b - a).try_normalize().unwrap_or(Vec2::X))
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svg(body: &str) -> String {
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">{body}</svg>"#)
    }

    fn animate(body: &str) -> Animation {
        Animation::parse(&svg(body)).unwrap().unwrap()
    }

    #[test]
    fn a_document_without_animation_has_none() {
        let still = svg(r#"<circle cx="50" cy="50" r="10"/>"#);
        assert!(Animation::parse(&still).unwrap().is_none());
    }

    #[test]
    fn an_animated_attribute_is_written_into_each_moment() {
        let animation = animate(
            r#"<circle r="5"><animate attributeName="r" from="10" to="20" dur="2s"/></circle>"#,
        );
        assert!(animation.at(0.0).contains(r#"<circle r="10">"#));
        assert!(animation.at(1.0).contains(r#"<circle r="15">"#));
        // Without fill="freeze" the attribute returns to its own value.
        assert!(animation.at(3.0).contains(r#"<circle r="5">"#));
    }

    #[test]
    fn an_attribute_the_element_lacks_is_added() {
        let animation = animate(
            r#"<rect width="9" height="9"/><rect width="5"><animate attributeName="opacity" values="0;1" dur="4s"/></rect>"#,
        );
        assert!(
            animation
                .at(1.0)
                .contains(r#"<rect width="9" height="9"/><rect opacity="0.25" width="5">"#)
        );
    }

    #[test]
    fn repeats_and_freezing_follow_the_timing_attributes() {
        let animation = animate(
            r#"<rect x="0"><animate attributeName="x" values="0;10" begin="1s" dur="2s" repeatCount="2" fill="freeze"/></rect>"#,
        );
        assert!(animation.at(0.5).contains(r#"x="0""#));
        assert!(animation.at(2.0).contains(r#"x="5""#));
        assert!(animation.at(4.0).contains(r#"x="5""#));
        assert!(animation.at(9.0).contains(r#"x="10""#));
    }

    #[test]
    fn a_negative_begin_starts_partway_through() {
        let animation = animate(
            r#"<rect x="0"><animate attributeName="x" values="0;100" begin="-1s" dur="4s" repeatCount="indefinite"/></rect>"#,
        );
        assert!(animation.at(0.0).contains(r#"x="25""#));
        assert!(animation.at(3.5).contains(r#"x="12.5""#));
    }

    #[test]
    fn animations_that_wait_for_an_event_never_start() {
        let still =
            svg(r#"<rect x="0"><animate attributeName="x" to="9" begin="click" dur="1s"/></rect>"#);
        assert!(Animation::parse(&still).unwrap().is_none());
    }

    #[test]
    fn colors_blend_and_unrelated_values_switch() {
        let animation = animate(
            r#"<rect fill="red"><animate attributeName="fill" values="black;#fff" dur="2s"/>
               <animate attributeName="stroke" values="url(#a);url(#b)" dur="2s"/></rect>"#,
        );
        let moment = animation.at(1.0);
        assert!(
            moment.contains(r#"fill="rgba(127.5,127.5,127.5,1)""#),
            "{moment}"
        );
        assert!(moment.contains(r#"stroke="url(#a)""#), "{moment}");
    }

    #[test]
    fn path_data_blends_number_by_number() {
        let animation = animate(
            r#"<path d="M0 0"><animate attributeName="d" values="M0 0 L10 0;M0 20 L10 40" dur="1s"/></path>"#,
        );
        assert!(animation.at(0.5).contains(r#"d="M0 10 L10 20""#));
    }

    #[test]
    fn discrete_values_hold_until_their_key_time() {
        let animation = animate(
            r#"<rect x="0"><animate attributeName="x" values="1;2;3" keyTimes="0;0.8;0.9" calcMode="discrete" dur="10s"/></rect>"#,
        );
        assert!(animation.at(7.9).contains(r#"x="1""#));
        assert!(animation.at(8.5).contains(r#"x="2""#));
        assert!(animation.at(9.5).contains(r#"x="3""#));
    }

    #[test]
    fn key_splines_ease_between_values() {
        assert!((ease([0.42, 0.0, 0.58, 1.0], 0.5) - 0.5).abs() < 1e-4);
        assert!(ease([0.42, 0.0, 1.0, 1.0], 0.5) < 0.4);
        let animation = animate(
            r#"<rect x="0"><animate attributeName="x" values="0;100" calcMode="spline" keySplines="0.42 0 1 1" dur="1s"/></rect>"#,
        );
        let moment = animation.at(0.5);
        let x: f32 = moment
            .split(r#"<rect x=""#)
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(x > 1.0 && x < 40.0, "{moment}");
    }

    #[test]
    fn transforms_replace_or_add_to_the_elements_own() {
        let animation = animate(
            r#"<g transform="translate(10 0)"><animateTransform attributeName="transform" type="rotate" from="0 5 5" to="90 5 5" dur="2s" additive="sum"/></g>
               <g transform="translate(10 0)"><animateTransform attributeName="transform" type="scale" to="3" dur="2s"/></g>"#,
        );
        let moment = animation.at(1.0);
        assert!(
            moment.contains(r#"transform="translate(10 0) rotate(45 5 5)""#),
            "{moment}"
        );
        assert!(moment.contains(r#"transform="scale(2 2)""#), "{moment}");
    }

    #[test]
    fn motion_follows_its_path_outside_the_elements_transform() {
        let animation = animate(
            r##"<rect transform="scale(2)"><animateMotion path="M0 0 H100 V100" dur="2s" rotate="auto"/></rect>
               <path id="track" d="M0 0 L0 40"/>
               <circle><animateMotion dur="1s"><mpath href="#track"/></animateMotion></circle>"##,
        );
        let moment = animation.at(1.5);
        assert!(
            moment.contains(r#"transform="translate(100 50) rotate(90) scale(2)""#),
            "{moment}"
        );
        assert!(
            animation
                .at(0.5)
                .contains(r#"<circle transform="translate(0 20)">"#)
        );
    }

    #[test]
    fn clock_values_read_their_units() {
        assert_eq!(clock("1.5s"), Some(1.5));
        assert_eq!(clock(" 250ms "), Some(0.25));
        assert_eq!(clock("2min"), Some(120.0));
        assert_eq!(clock("0:01:30"), Some(90.0));
        assert_eq!(clock("-2"), Some(-2.0));
        assert_eq!(clock("click"), None);
    }
}
