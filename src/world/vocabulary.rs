use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::Activity;

/// Evidence a word's strongest meaning needs before the ghost acts on it.
const LEARNED: f32 = 2.0;
/// The share of a word's evidence its strongest meaning needs, so words
/// heard with everything never come to mean one thing.
const CLEAR: f32 = 0.6;
/// Being told no makes a meaning unlikely, but never more unlikely than
/// this.
const DOUBT: f32 = -3.0;
/// Words kept at most. The least heard words without a meaning go first.
const WORDS_KEPT: usize = 400;
/// Words taken from one message at most.
const WORDS_PER_MESSAGE: usize = 12;
/// Characters a word may have at most.
const LONGEST: usize = 24;

/// How something was said, which the ghost reads without knowing any
/// words, the way a pet does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Greeting,
    Farewell,
    Praise,
    Scolding,
}

// Words of tone, in English and Korean, and little words that carry no
// meaning of their own. Words are compared lowercased and without
// apostrophes.
const GREETINGS: &[&str] = &[
    "hi",
    "hello",
    "hey",
    "heya",
    "hiya",
    "howdy",
    "yo",
    "morning",
    "안녕",
    "안녕하세요",
    "하이",
    "헬로",
    "반가워",
];
const FAREWELLS: &[&str] = &[
    "bye",
    "goodbye",
    "goodnight",
    "night",
    "cya",
    "farewell",
    "잘자",
    "잘가",
    "바이",
    "안녕히",
];
const PRAISE: &[&str] = &[
    "good",
    "great",
    "nice",
    "yes",
    "yay",
    "yeah",
    "yep",
    "wow",
    "cool",
    "clever",
    "smart",
    "cute",
    "adorable",
    "sweet",
    "love",
    "lovely",
    "awesome",
    "amazing",
    "bravo",
    "perfect",
    "beautiful",
    "thanks",
    "thank",
    "좋아",
    "좋아요",
    "좋다",
    "잘했어",
    "잘했다",
    "잘했어요",
    "최고",
    "최고야",
    "귀여워",
    "귀엽다",
    "사랑해",
    "고마워",
    "고마워요",
    "착해",
    "착하다",
    "예뻐",
    "예쁘다",
    "멋져",
    "멋지다",
    "우와",
    "응",
    "그래",
    "맞아",
];
const SCOLDING: &[&str] = &[
    "no",
    "nope",
    "bad",
    "stop",
    "dont",
    "naughty",
    "quit",
    "enough",
    "안돼",
    "아니",
    "아니야",
    "그만",
    "하지마",
    "나빠",
    "싫어",
];
const LITTLE: &[&str] = &[
    "a", "an", "the", "to", "and", "or", "of", "in", "on", "at", "for", "with", "is", "are", "am",
    "was", "be", "it", "its", "this", "that", "these", "those", "im", "me", "my", "you", "your",
    "youre", "we", "us", "our", "do", "does", "did", "can", "could", "will", "would", "should",
    "please", "now", "just", "so", "too", "very", "really", "oh", "ok", "okay", "lets", "let",
    "what", "how", "why", "when", "who", "want", "wanna", "like", "some", "any", "again", "more",
    "then", "if", "but", "all", "go", "get", "have", "has", "been", "bit", "little", "about",
    "from", "by", "as", "into", "there", "not", "ghost", "job", "done", "work", "boy", "girl",
    "one", "thing", "좀", "너", "나", "우리", "그냥", "진짜", "정말", "이제", "한번", "빨리", "해",
    "해줘", "줘",
];

// Ordinary states and sentence openings that are not names in an ambiguous introduction.
const ORDINARY: &[&str] = &[
    "tired",
    "back",
    "home",
    "here",
    "sorry",
    "fine",
    "good",
    "ok",
    "okay",
    "happy",
    "sad",
    "bored",
    "hungry",
    "sleepy",
    "busy",
    "done",
    "leaving",
    "going",
    "so",
    "very",
    "just",
    "not",
    "a",
    "an",
    "the",
    "and",
    "well",
    "great",
    "excited",
    "ready",
    "awake",
    "alone",
    "lost",
    "late",
    "early",
    "sick",
    "ill",
    "cold",
    "hot",
    "afraid",
    "scared",
    "confused",
    "curious",
    "new",
    "sure",
    "still",
    "really",
    "feeling",
    "looking",
    "trying",
    "playing",
    "watching",
    "waiting",
    "having",
    "doing",
    "getting",
    "only",
    "also",
    "finally",
    "almost",
    "never",
    "always",
    "from",
    "in",
    "on",
    "at",
    "fun",
    "glad",
    "alive",
    "dead",
    "reading",
    "writing",
    "thinking",
    "talking",
    "super",
    "quite",
    "lonely",
    "content",
    "grumpy",
    "annoyed",
    "proud",
    "asleep",
    "stuck",
    "interested",
    "off",
    "out",
    "inside",
    "outside",
];

/// What the ghost makes of a message without knowing what its words mean
/// yet: the tone, the words that might mean something, and whether it was
/// called by name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reading {
    pub tone: Option<Tone>,
    /// The message's words, without words of tone, little words, or the
    /// ghost's name, in the order said.
    pub words: Vec<String>,
    pub named: bool,
}

/// Reads `text` said to a ghost called `name`.
pub fn read(text: &str, name: &str) -> Reading {
    let name = tokens(name);
    let mut tones = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut named = false;
    for token in tokens(text) {
        if !name.is_empty() && name.contains(&token) {
            named = true;
            continue;
        }
        let tone = [
            (GREETINGS, Tone::Greeting),
            (FAREWELLS, Tone::Farewell),
            (PRAISE, Tone::Praise),
            (SCOLDING, Tone::Scolding),
        ]
        .into_iter()
        .find(|(list, _)| list.contains(&token.as_str()));
        if let Some((_, tone)) = tone {
            tones.push(tone);
        } else if !LITTLE.contains(&token.as_str())
            && !words.contains(&token)
            && words.len() < WORDS_PER_MESSAGE
        {
            words.push(token);
        }
    }
    // Being told no outweighs everything, and a goodbye or a hello says
    // more than a kind word on the way.
    let tone = [Tone::Scolding, Tone::Farewell, Tone::Greeting, Tone::Praise]
        .into_iter()
        .find(|tone| tones.contains(tone));
    Reading { tone, words, named }
}

/// Finds an introduction without guessing a name from an ordinary sentence.
pub fn introduced_name(text: &str) -> Option<String> {
    let original: Vec<&str> = text
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|character: char| {
                !character.is_alphanumeric() && !matches!(character, '\'' | '’')
            })
        })
        .filter(|word| !word.is_empty())
        .collect();
    let words: Vec<String> = original
        .iter()
        .map(|word| word.to_lowercase().replace('’', "'"))
        .collect();
    for index in 0..words.len() {
        let tail = &words[index..];
        let starts = |phrase: &[&str]| {
            tail.iter()
                .map(String::as_str)
                .take(phrase.len())
                .eq(phrase.iter().copied())
        };
        let (offset, cautious) = if starts(&["my", "name", "is"]) {
            (3, false)
        } else if starts(&["call", "me"]) || starts(&["내", "이름은"]) || starts(&["제", "이름은"])
        {
            (2, false)
        } else if starts(&["i", "am"]) {
            (2, true)
        } else if starts(&["i'm"]) || starts(&["im"]) {
            (1, true)
        } else if starts(&["this", "is"])
            && words[..index]
                .iter()
                .any(|word| GREETINGS.contains(&word.as_str()))
        {
            (2, false)
        } else if tail.first().is_some_and(|word| word.ends_with("라고"))
            && tail
                .get(1)
                .is_some_and(|word| matches!(word.as_str(), "불러" | "불러요" | "불러주세요"))
        {
            (0, false)
        } else {
            continue;
        };
        let Some(&candidate) = original.get(index + offset) else {
            continue;
        };
        if cautious || starts(&["this", "is"]) {
            let lower = candidate.to_lowercase();
            if (cautious && !candidate.chars().next().is_some_and(char::is_uppercase))
                || ORDINARY.contains(&lower.as_str())
                || LITTLE.contains(&lower.as_str())
            {
                continue;
            }
        }
        if let Some(name) = clean_name(candidate) {
            return Some(name);
        }
    }
    None
}

// Words after "call you" that make a promise to phone or a pet name, not a name.
const NOT_NAMES: &[&str] = &[
    "later",
    "tomorrow",
    "tonight",
    "today",
    "soon",
    "back",
    "sometime",
    "after",
    "before",
    "maybe",
    "cute",
    "silly",
    "sweet",
    "sweetie",
    "honey",
    "buddy",
    "friend",
    "mine",
    "names",
    "something",
    "anything",
    "nothing",
    "whatever",
    "뭐",
    "뭐야",
    "뭐니",
    "뭐에요",
    "뭐예요",
    "뭘까",
    "무엇",
    "누구",
    "누구야",
    "뭐라고",
];

/// Finds the viewer giving the ghost a name ("your name is Pip", "I'll call
/// you Pip", "너의 이름은 뽀삐야", "뽀삐라고 부를게") without guessing one
/// from an ordinary sentence. A question about its name gives none.
pub fn given_name(text: &str) -> Option<String> {
    let original: Vec<&str> = text
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|character: char| {
                !character.is_alphanumeric() && !matches!(character, '\'' | '’')
            })
        })
        .filter(|word| !word.is_empty())
        .collect();
    let words: Vec<String> = original
        .iter()
        .map(|word| word.to_lowercase().replace('’', "'"))
        .collect();
    let asking = text.contains('?');
    for index in 0..words.len() {
        let tail = &words[index..];
        let starts = |phrase: &[&str]| {
            tail.iter()
                .map(String::as_str)
                .take(phrase.len())
                .eq(phrase.iter().copied())
        };
        // "Can I call you Pip?" asks to name it; "your name is Pip?" asks
        // what its name is.
        let offset = if starts(&["call", "you"])
            || starts(&["name", "you"])
            || starts(&["naming", "you"])
            || starts(&["you're", "called"])
            || starts(&["you're", "named"])
        {
            2
        } else if starts(&["you", "are", "called"]) || starts(&["you", "are", "named"]) {
            3
        } else if tail.first().is_some_and(|word| word.ends_with("라고"))
            && tail.get(1).is_some_and(|word| {
                [
                    "부를게",
                    "부를께",
                    "부를래",
                    "부를거야",
                    "부르자",
                    "불러줄게",
                    "할게",
                ]
                .contains(&word.as_str())
                    || (word.starts_with("불러도") && asking)
            })
        {
            0
        } else if asking {
            continue;
        } else if starts(&["your", "name", "is"]) {
            3
        } else if starts(&["your", "new", "name", "is"]) {
            4
        } else if starts(&["your", "name's"])
            || ["너의", "네", "니", "너"]
                .iter()
                .any(|you| starts(&[you, "이름은"]))
            || starts(&["넌", "이제"])
            || starts(&["너는", "이제"])
            || starts(&["이제", "넌"])
            || starts(&["이제부터", "넌"])
            || starts(&["넌", "이제부터"])
        {
            2
        } else {
            continue;
        };
        let Some(&candidate) = original.get(index + offset) else {
            continue;
        };
        let lower = candidate.to_lowercase();
        if ORDINARY.contains(&lower.as_str())
            || LITTLE.contains(&lower.as_str())
            || NOT_NAMES.contains(&lower.as_str())
        {
            continue;
        }
        if let Some(name) = clean_name(candidate)
            && !NOT_NAMES.contains(&name.to_lowercase().as_str())
        {
            return Some(name);
        }
    }
    None
}

/// A single name, kept safe for saves, milestones, and terminal text.
pub(super) fn clean_name(text: &str) -> Option<String> {
    let mut name = text.trim_matches(|character: char| !character.is_alphanumeric());
    for particle in ["이에요", "이라고", "입니다", "예요", "이야", "라고", "야"] {
        // A one-syllable stem keeps the "이" of a shorter particle, as in
        // "콩이야" for 콩이.
        if let Some(stem) = name.strip_suffix(particle)
            && stem.chars().count() >= 2
        {
            name = stem;
            break;
        }
    }
    if !(2..=16).contains(&name.chars().count()) || !name.chars().all(char::is_alphabetic) {
        return None;
    }
    let mut characters = name.chars();
    let first = characters.next()?;
    if first.is_ascii_alphabetic() || ('À'..='ɏ').contains(&first) || ('Ḁ'..='ỿ').contains(&first)
    {
        Some(first.to_uppercase().chain(characters).collect())
    } else {
        Some(name.to_owned())
    }
}

/// The words of `text`: lowercased, without apostrophes, split at anything
/// but letters, and without numbers, single Latin letters, or Korean object
/// markers.
pub fn tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .replace(['\'', '’'], "")
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty() && !token.chars().any(char::is_numeric))
        .filter(|token| !(token.is_ascii() && token.len() < 2))
        .map(|token| {
            let syllables: Vec<char> = token.chars().collect();
            let hangul = syllables
                .iter()
                .all(|syllable| ('가'..='힣').contains(syllable));
            match syllables.split_last() {
                Some((last, stem)) if hangul && !stem.is_empty() && matches!(last, '을' | '를') => {
                    stem.iter().collect()
                }
                _ => token.to_owned(),
            }
        })
        .filter(|token: &String| token.chars().count() <= LONGEST)
        .collect()
}

/// A word the ghost has heard, and what it has come to think it means.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Word {
    /// Times it was heard.
    pub heard: u32,
    /// How much it seems to mean each activity, by the activity's name:
    /// positive when it went with the activity, negative when the viewer
    /// said no to it.
    pub meanings: BTreeMap<String, f32>,
}

impl Word {
    /// What the word means, once its strongest meaning has enough evidence
    /// and clearly more than the others.
    pub fn meaning(&self) -> Option<Activity> {
        let total: f32 = self
            .meanings
            .values()
            .filter(|evidence| **evidence > 0.0)
            .sum();
        let (name, evidence) = self
            .meanings
            .iter()
            .max_by(|first, second| first.1.total_cmp(second.1))?;
        if *evidence >= LEARNED && *evidence >= CLEAR * total {
            Activity::parse(name)
        } else {
            None
        }
    }

    pub fn evidence(&self, activity: Activity) -> f32 {
        self.meanings.get(activity.name()).copied().unwrap_or(0.0)
    }
}

/// The words the ghost has picked up from the viewer. It learns them the
/// way a pet does: from what it was doing when it heard them, what the
/// viewer did next, and whether the viewer was pleased with what it did.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Vocabulary {
    pub words: BTreeMap<String, Word>,
}

impl Vocabulary {
    /// Counts that `words` were heard once more.
    pub fn hear(&mut self, words: &[String]) {
        for word in words {
            let entry = self.words.entry(word.clone()).or_default();
            entry.heard = entry.heard.saturating_add(1);
        }
        self.forget(words);
    }

    /// Times `word` was heard.
    pub fn heard(&self, word: &str) -> u32 {
        self.words.get(word).map_or(0, |word| word.heard)
    }

    /// Adds `amount` of evidence, negative to take it away, that `words`
    /// mean `activity`. Returns the words that just came to mean it.
    pub fn teach(&mut self, words: &[String], activity: Activity, amount: f32) -> Vec<String> {
        let mut learned = Vec::new();
        for word in words {
            let entry = self.words.entry(word.clone()).or_default();
            let before = entry.meaning();
            let evidence = entry
                .meanings
                .entry(activity.name().to_owned())
                .or_default();
            *evidence = (*evidence + amount).max(DOUBT);
            let after = entry.meaning();
            if after == Some(activity) && before != after {
                learned.push(word.clone());
            }
        }
        self.forget(words);
        learned
    }

    /// What `words` ask for together: the meaning with the most evidence
    /// among the words that have one, and the word that carries it.
    pub fn meaning(&self, words: &[String]) -> Option<(String, Activity)> {
        words
            .iter()
            .filter_map(|word| {
                let entry = self.words.get(word)?;
                let activity = entry.meaning()?;
                Some((word, activity, entry.evidence(activity)))
            })
            .max_by(|first, second| first.2.total_cmp(&second.2))
            .map(|(word, activity, _)| (word.clone(), activity))
    }

    /// How much `words` together seem to mean `activity`.
    pub fn evidence(&self, words: &[String], activity: Activity) -> f32 {
        words
            .iter()
            .filter_map(|word| self.words.get(word))
            .map(|word| word.evidence(activity))
            .sum()
    }

    /// The words it knows and what they mean, the most heard first.
    pub fn known(&self) -> Vec<(&str, Activity)> {
        let mut known: Vec<(&str, Activity, u32)> = self
            .words
            .iter()
            .filter_map(|(text, word)| Some((text.as_str(), word.meaning()?, word.heard)))
            .collect();
        known.sort_by(|first, second| second.2.cmp(&first.2).then(first.0.cmp(second.0)));
        known
            .into_iter()
            .map(|(text, activity, _)| (text, activity))
            .collect()
    }

    /// The vocabulary as saved, without evidence that isn't a number or
    /// words that couldn't have been heard, so a damaged save can't break
    /// it.
    pub fn sanitized(mut self) -> Self {
        self.words
            .retain(|text, _| tokens(text).as_slice() == [text.clone()]);
        for word in self.words.values_mut() {
            word.meanings.retain(|_, evidence| evidence.is_finite());
            for evidence in word.meanings.values_mut() {
                *evidence = evidence.max(DOUBT);
            }
        }
        self.forget(&[]);
        self
    }

    // Keeps the vocabulary within its size, letting go of the least heard
    // words that mean nothing yet, but never of `keeping`.
    fn forget(&mut self, keeping: &[String]) {
        while self.words.len() > WORDS_KEPT {
            let Some(least) = self
                .words
                .iter()
                .filter(|(text, word)| word.meaning().is_none() && !keeping.contains(text))
                .min_by_key(|(_, word)| word.heard)
                .map(|(text, _)| text.clone())
            else {
                return;
            };
            self.words.remove(&least);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        tokens(text)
    }

    #[test]
    fn messages_split_into_lowercase_words_without_numbers_or_object_markers() {
        assert_eq!(
            tokens("Don't STOP, Boo! 3 balls? 공을 가져와 x"),
            ["dont", "stop", "boo", "balls", "공", "가져와"]
        );
        assert_eq!(tokens("을 그림을"), ["을", "그림"]);
        assert!(tokens(&"long".repeat(10)).is_empty());
    }

    #[test]
    fn tone_is_read_without_knowing_any_words_and_no_outweighs_the_rest() {
        assert_eq!(read("Good job!", "Boo").tone, Some(Tone::Praise));
        assert_eq!(read("hello there", "Boo").tone, Some(Tone::Greeting));
        assert_eq!(read("good night", "Boo").tone, Some(Tone::Farewell));
        assert_eq!(read("no, not good", "Boo").tone, Some(Tone::Scolding));
        assert_eq!(read("잘했어!", "Boo").tone, Some(Tone::Praise));
        assert_eq!(read("dance", "Boo").tone, None);
    }

    #[test]
    fn a_reading_keeps_the_words_that_might_mean_something_and_notices_its_name() {
        let reading = read("Hey Boo, can you dance for me? dance!", "Boo");
        assert_eq!(reading.words, ["dance"]);
        assert!(reading.named);
        assert!(!read("dance", "Boo").named);
        let reading = read("Mister Moonbeam, the ball", "Mister Moonbeam");
        assert_eq!(reading.words, ["ball"]);
        assert!(reading.named);
    }

    #[test]
    fn a_word_means_something_once_its_evidence_is_strong_and_clear() {
        let mut vocabulary = Vocabulary::default();
        let dance = words("dance");
        assert!(vocabulary.teach(&dance, Activity::Dance, 1.0).is_empty());
        assert_eq!(vocabulary.meaning(&dance), None);
        assert_eq!(vocabulary.teach(&dance, Activity::Dance, 1.0), ["dance"]);
        assert_eq!(
            vocabulary.meaning(&dance),
            Some(("dance".to_owned(), Activity::Dance))
        );
        // Learning it again isn't news.
        assert!(vocabulary.teach(&dance, Activity::Dance, 1.0).is_empty());
    }

    #[test]
    fn words_heard_with_everything_never_come_to_mean_one_thing() {
        let mut vocabulary = Vocabulary::default();
        let then = words("then");
        for activity in [Activity::Play, Activity::Dance, Activity::Rest] {
            vocabulary.teach(&then, activity, 2.5);
        }
        assert_eq!(vocabulary.meaning(&then), None);
    }

    #[test]
    fn being_told_no_unlearns_a_meaning_but_only_so_far() {
        let mut vocabulary = Vocabulary::default();
        let ball = words("ball");
        vocabulary.teach(&ball, Activity::Dance, 2.0);
        assert!(vocabulary.meaning(&ball).is_some());
        for _ in 0..10 {
            vocabulary.teach(&ball, Activity::Dance, -1.5);
        }
        assert_eq!(vocabulary.meaning(&ball), None);
        assert_eq!(vocabulary.evidence(&ball, Activity::Dance), DOUBT);
        assert_eq!(vocabulary.teach(&ball, Activity::Fetch, 2.0), ["ball"]);
        assert_eq!(vocabulary.meaning(&ball).unwrap().1, Activity::Fetch);
    }

    #[test]
    fn the_strongest_known_word_carries_a_message() {
        let mut vocabulary = Vocabulary::default();
        vocabulary.teach(&words("ball"), Activity::Fetch, 4.0);
        vocabulary.teach(&words("sleep"), Activity::Rest, 2.5);
        vocabulary.teach(&words("maybe"), Activity::Dance, 1.0);
        assert_eq!(
            vocabulary.meaning(&words("sleep ball maybe")),
            Some(("ball".to_owned(), Activity::Fetch))
        );
        assert_eq!(vocabulary.meaning(&words("maybe")), None);
    }

    #[test]
    fn known_words_are_listed_most_heard_first() {
        let mut vocabulary = Vocabulary::default();
        vocabulary.teach(&words("ball"), Activity::Fetch, 2.0);
        vocabulary.teach(&words("dance"), Activity::Dance, 2.0);
        vocabulary.hear(&words("dance hmm"));
        vocabulary.hear(&words("dance"));
        assert_eq!(
            vocabulary.known(),
            [("dance", Activity::Dance), ("ball", Activity::Fetch)]
        );
        assert_eq!(vocabulary.heard("dance"), 2);
        assert_eq!(vocabulary.heard("hmm"), 1);
    }

    #[test]
    fn the_vocabulary_forgets_the_least_heard_unknown_words_first() {
        let mut vocabulary = Vocabulary::default();
        vocabulary.teach(&words("ball"), Activity::Fetch, 2.0);
        vocabulary.hear(&words("often often"));
        let many: Vec<String> = (0..WORDS_KEPT)
            .map(|index| {
                format!("word{}", char::from(b'a' + (index % 26) as u8)) + &"x".repeat(index / 26)
            })
            .collect();
        for word in &many {
            vocabulary.hear(std::slice::from_ref(word));
        }
        assert_eq!(vocabulary.words.len(), WORDS_KEPT);
        assert!(vocabulary.words.contains_key("ball"));
        assert!(vocabulary.words.contains_key("often"));
    }

    #[test]
    fn a_damaged_vocabulary_is_brought_back_in_range() {
        let mut vocabulary = Vocabulary::default();
        let mut word = Word {
            heard: 3,
            ..Word::default()
        };
        word.meanings.insert("dance".to_owned(), f32::NAN);
        word.meanings.insert("play".to_owned(), -50.0);
        word.meanings.insert("fetch".to_owned(), 5.0);
        vocabulary.words.insert("ball".to_owned(), word);
        vocabulary
            .words
            .insert("Not A Word".to_owned(), Word::default());
        let vocabulary = vocabulary.sanitized();
        assert_eq!(vocabulary.words.len(), 1);
        let ball = &vocabulary.words["ball"];
        assert_eq!(ball.evidence(Activity::Dance), 0.0);
        assert_eq!(ball.evidence(Activity::Play), DOUBT);
        assert_eq!(ball.meaning(), Some(Activity::Fetch));
    }

    #[test]
    fn introductions_find_names_in_english_and_korean_and_keep_their_spelling() {
        for (text, expected) in [
            ("my name is mina.", "Mina"),
            ("My name is Alex!", "Alex"),
            ("Please call me sam.", "Sam"),
            ("I'm Mina", "Mina"),
            ("I’m Mina!", "Mina"),
            ("i am Sam", "Sam"),
            ("im Jo", "Jo"),
            ("I'm Élodie.", "Élodie"),
            ("hello, this is Mina.", "Mina"),
            ("Hi there, this is Alex!", "Alex"),
            ("내 이름은 민수야", "민수"),
            ("제 이름은 지민이에요.", "지민"),
            ("내 이름은 수빈이야!", "수빈"),
            ("제 이름은 미나예요", "미나"),
            ("내 이름은 지민입니다", "지민"),
            ("민수라고 불러", "민수"),
            ("지민이라고 불러!", "지민"),
            ("미나라고 불러요.", "미나"),
            ("내 이름은 마야", "마야"),
            ("my name is 佐藤", "佐藤"),
            ("my name is елена", "елена"),
            ("my name is abcdefghijklmnop", "Abcdefghijklmnop"),
        ] {
            assert_eq!(introduced_name(text).as_deref(), Some(expected), "{text}");
        }
    }

    #[test]
    fn ordinary_sentences_and_invalid_names_are_not_introductions() {
        for text in [
            "I'm tired",
            "I'm Tired",
            "I'm Back!",
            "I am Home",
            "im mina",
            "I'm happy",
            "I'm Sorry",
            "I'm Fine",
            "I'm Good",
            "I'm Ok",
            "I'm Okay",
            "I'm Happy",
            "I'm Sad",
            "I'm Bored",
            "I'm Hungry",
            "I'm Sleepy",
            "I'm Busy",
            "I'm Done",
            "I'm Leaving",
            "I'm Going",
            "I'm So happy",
            "I'm Very tired",
            "I'm Just here",
            "I'm Not Mina",
            "I'm A ghost",
            "I'm An owl",
            "I'm The viewer",
            "I'm And",
            "I'm Feeling good",
            "I am Looking around",
            "I am here",
            "this is Mina",
            "hi, this is fun",
            "hello, this is my ball",
            "this is a ball",
            "Hi, this ball is Mina",
            "call the ball Mina",
            "my name is X",
            "my name is Alex123",
            "call me abcdefghijklmnopq",
            "my name is",
            "call me 123",
            "내 이름은",
            "내 이름은 123",
            "민수라고 말했어",
            "안녕",
            "hello Mina",
            "내 이름은 민1수",
        ] {
            assert_eq!(introduced_name(text), None, "{text}");
        }
    }

    #[test]
    fn the_viewer_can_name_the_ghost_in_english_and_korean() {
        for (text, expected) in [
            ("your name is pip", "Pip"),
            ("Your name is Pip!", "Pip"),
            ("your new name is Mochi.", "Mochi"),
            ("your name's Wisp", "Wisp"),
            ("I'll call you pip", "Pip"),
            ("i will call you Bean", "Bean"),
            ("let's call you Pip", "Pip"),
            ("Can I call you Pip?", "Pip"),
            ("I name you Mochi", "Mochi"),
            ("i'm naming you Mochi", "Mochi"),
            ("you're called Pip now", "Pip"),
            ("you are named Wisp", "Wisp"),
            ("너의 이름은 뽀삐야", "뽀삐"),
            ("네 이름은 콩이야!", "콩이"),
            ("너 이름은 모찌", "모찌"),
            ("넌 이제 뽀삐야", "뽀삐"),
            ("이제부터 넌 콩이야", "콩이"),
            ("뽀삐라고 부를게", "뽀삐"),
            ("모찌라고 할게!", "모찌"),
            ("콩이라고 불러도 돼?", "콩이"),
        ] {
            assert_eq!(given_name(text).as_deref(), Some(expected), "{text}");
        }
    }

    #[test]
    fn questions_promises_and_introductions_do_not_name_the_ghost() {
        for text in [
            "what is your name?",
            "your name is Pip?",
            "is your name Boo",
            "I'll call you later",
            "I'll call you tomorrow!",
            "call you back soon",
            "I'll call you cute",
            "my name is Mina",
            "call me Sam",
            "너 이름이 뭐야?",
            "너의 이름은 뭐야",
            "너의 이름은?",
            "민수라고 불러",
            "넌 바보야",
            "your name is",
            "your name is Pip123",
            "you are cute",
        ] {
            assert_eq!(given_name(text), None, "{text}");
        }
    }
}
