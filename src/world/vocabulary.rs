//! Reading a message at a glance: how it was said, and whether it gives a
//! name. What it means is for the minds to understand.

/// Characters a word may have at most.
const LONGEST: usize = 24;

/// How something was said, read at a glance.
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
    "better",
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
    "어",
    "괜찮아",
    "나아졌어",
    "그래",
    "맞아",
];
const SCOLDING: &[&str] = &[
    "no",
    "nope",
    "별로",
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

/// How a message sounds, and whether it called the ghost by name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reading {
    pub tone: Option<Tone>,
    pub named: bool,
}

/// Reads `text` said to a ghost called `name`.
pub fn read(text: &str, name: &str) -> Reading {
    if tokens(text) == ["not", "really"] {
        return Reading {
            tone: Some(Tone::Scolding),
            named: false,
        };
    }
    let name = tokens(name);
    let mut tones = Vec::new();
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
        }
    }
    // Being told no outweighs everything, and a goodbye or a hello says
    // more than a kind word on the way.
    let tone = [Tone::Scolding, Tone::Farewell, Tone::Greeting, Tone::Praise]
        .into_iter()
        .find(|tone| tones.contains(tone));
    Reading { tone, named }
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
    "weird",
    "funny",
    "nice",
    "pretty",
    "beautiful",
    "strange",
    "boring",
    "long",
    "short",
    "perfect",
    "great",
    "awesome",
    "lovely",
    "stupid",
    "dumb",
    "cool",
    "adorable",
    "wrong",
    "right",
    "예뻐",
    "이상해",
    "귀여워",
    "좋아",
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

// Words before "call you" that deny the name or ask about it instead.
const NOT_NAMING: &[&str] = &[
    "not",
    "never",
    "no",
    "don't",
    "dont",
    "won't",
    "wont",
    "can't",
    "cant",
    "cannot",
    "shouldn't",
    "wouldn't",
    "didn't",
    "doesn't",
    "why",
    "who",
    "what",
    "where",
    "when",
    "how",
    "they",
    "people",
    "everyone",
    "does",
    "did",
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
        let denied = || {
            words[..index]
                .iter()
                .any(|word| NOT_NAMING.contains(&word.as_str()))
        };
        let offset = if (starts(&["call", "you"])
            || starts(&["name", "you"])
            || starts(&["naming", "you"])
            || starts(&["you're", "called"])
            || starts(&["you're", "named"]))
            && !denied()
        {
            2
        } else if (starts(&["you", "are", "called"]) || starts(&["you", "are", "named"]))
            && !denied()
        {
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn a_reading_notices_its_name() {
        assert!(read("Hey Boo, can you dance for me?", "Boo").named);
        assert!(!read("dance", "Boo").named);
        assert!(read("Mister Moonbeam, the ball", "Mister Moonbeam").named);
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
            "넌 이제 자야 해",
            "넌 이제 혼자야",
            "your name is weird",
            "your name is so cute",
            "네 이름은 예뻐",
            "your name is",
            "your name is Pip123",
            "you are cute",
            "I will not call you Pip",
            "I'd never call you Pip",
            "Why do they call you Pip?",
            "who are you called after?",
            "people call you Pip",
        ] {
            assert_eq!(given_name(text), None, "{text}");
        }
    }
}
