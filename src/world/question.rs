/// Something the viewer asked that the ghost can answer from what it knows
/// of itself, without a model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Question {
    /// How it feels.
    Feeling,
    /// What it is doing.
    Doing,
    /// What it is called.
    Name,
    /// Whether it knows the viewer's name.
    ViewerName,
    /// Whether it likes the viewer.
    Liking,
    /// How old it is.
    Age,
    /// What it likes doing most.
    Favorite,
    /// What the weather is like outside.
    Weather,
    /// Whether it is tired.
    Tired,
    /// Whether it is hungry.
    Hungry,
    /// Whether it is bored.
    Bored,
    /// Whether it is lonely or missed the viewer.
    Lonely,
    /// What it keeps on its shelf.
    Shelf,
    /// How it gets on with the bird at the window.
    Bird,
    Friend,
    /// A story from its life.
    Story,
    Away,
    Constellations,
    Wearing,
    Reading,
    FavoriteBook,
    ReadAloud,
    Music,
    Songs,
    Wish,
    Mirror,
}

impl Question {
    pub fn ask(text: &str) -> Option<Self> {
        ask(text)
    }
}

// Questions in English, as runs of words to find in the message, compared
// lowercased and without apostrophes. More particular questions come first,
// so "are you tired" is not taken for "are you okay".
const ENGLISH: &[(Question, &[&[&str]])] = &[
    (
        Question::Mirror,
        &[
            &["who", "is", "in", "the", "mirror"],
            &["whos", "in", "the", "mirror"],
            &["whos", "that", "in", "the", "mirror"],
            &["who", "is", "that", "in", "the", "mirror"],
        ],
    ),
    (
        Question::Wearing,
        &[
            &["what", "are", "you", "wearing"],
            &["whats", "that", "youre", "wearing"],
        ],
    ),
    (
        Question::ReadAloud,
        &[&["read", "to", "me"], &["read", "me", "a", "book"]],
    ),
    (
        Question::Reading,
        &[
            &["what", "are", "you", "reading"],
            &["what", "book", "are", "you", "reading"],
        ],
    ),
    (
        Question::FavoriteBook,
        &[
            &["whats", "your", "favorite", "book"],
            &["what", "is", "your", "favorite", "book"],
            &["whats", "your", "favourite", "book"],
            &["what", "is", "your", "favourite", "book"],
        ],
    ),
    (
        Question::Music,
        &[
            &["do", "you", "like", "music"],
            &["do", "you", "like", "songs"],
        ],
    ),
    (
        Question::Songs,
        &[
            &["what", "songs", "do", "you", "know"],
            &["what", "tunes", "do", "you", "know"],
        ],
    ),
    (
        Question::Wish,
        &[
            &["what", "do", "you", "wish", "for"],
            &["do", "you", "have", "a", "wish"],
            &["whats", "your", "wish"],
            &["what", "is", "your", "wish"],
        ],
    ),
    (
        Question::Story,
        &[
            &["tell", "me", "a", "story"],
            &["tell", "a", "story"],
            &["story", "please"],
            &["bedtime", "story"],
        ],
    ),
    (
        Question::Away,
        &[
            &["what", "did", "you", "do", "while", "i", "was", "away"],
            &["what", "did", "you", "do", "while", "i", "was", "gone"],
            &["what", "were", "you", "doing"],
            &["what", "did", "you", "do", "without", "me"],
        ],
    ),
    (
        Question::Constellations,
        &[
            &["do", "you", "know", "any", "constellations"],
            &["do", "you", "know", "constellations"],
            &["what", "constellations"],
            &["have", "you", "found", "any", "constellations"],
            &["what", "are", "those", "stars"],
            &["what", "are", "the", "stars"],
            &["do", "you", "know", "the", "stars"],
            &["do", "you", "know", "any", "stars"],
            &["what", "stars", "do", "you", "know"],
        ],
    ),
    (
        Question::Friend,
        &[
            &["who", "is", "your", "friend"],
            &["whos", "your", "friend"],
            &["how", "is", "your", "friend"],
            &["do", "you", "have", "a", "friend"],
            &["tell", "me", "about", "your", "friend"],
            &["where", "is", "your", "friend"],
            &["whats", "your", "friends", "name"],
            &["what", "is", "your", "friends", "name"],
        ],
    ),
    (
        Question::Shelf,
        &[
            &["whats", "on", "your", "shelf"],
            &["what", "is", "on", "your", "shelf"],
            &["whats", "on", "the", "shelf"],
            &["what", "are", "your", "keepsakes"],
            &["what", "are", "your", "treasures"],
        ],
    ),
    (
        Question::Bird,
        &[
            &["do", "you", "like", "the", "bird"],
            &["do", "you", "like", "birds"],
            &["is", "the", "bird", "your", "friend"],
            &["are", "you", "friends", "with", "the", "bird"],
        ],
    ),
    (
        Question::ViewerName,
        &[
            &["whats", "my", "name"],
            &["what", "is", "my", "name"],
            &["do", "you", "know", "my", "name"],
            &["remember", "my", "name"],
            &["who", "am", "i"],
        ],
    ),
    (
        Question::Name,
        &[
            &["whats", "your", "name"],
            &["what", "is", "your", "name"],
            &["who", "are", "you"],
            &["your", "name", "is"],
        ],
    ),
    (
        Question::Age,
        &[&["how", "old", "are", "you"], &["how", "old", "r", "u"]],
    ),
    (
        Question::Liking,
        &[
            &["do", "you", "like", "me"],
            &["do", "you", "love", "me"],
            &["are", "we", "friends"],
            &["do", "u", "like", "me"],
            &["do", "u", "love", "me"],
        ],
    ),
    (
        Question::Favorite,
        &[
            &["whats", "your", "favorite"],
            &["whats", "your", "favourite"],
            &["what", "is", "your", "favorite"],
            &["what", "do", "you", "like"],
            &["what", "do", "you", "love"],
            &["what", "do", "you", "enjoy"],
        ],
    ),
    (
        Question::Weather,
        &[
            &["hows", "the", "weather"],
            &["whats", "the", "weather"],
            &["what", "is", "the", "weather"],
            &["how", "is", "the", "weather"],
            &["is", "it", "raining"],
            &["is", "it", "snowing"],
        ],
    ),
    (
        Question::Tired,
        &[
            &["are", "you", "tired"],
            &["are", "you", "sleepy"],
            &["r", "u", "tired"],
            &["you", "tired"],
            &["you", "sleepy"],
        ],
    ),
    (
        Question::Hungry,
        &[
            &["are", "you", "hungry"],
            &["you", "hungry"],
            &["r", "u", "hungry"],
        ],
    ),
    (
        Question::Bored,
        &[
            &["are", "you", "bored"],
            &["you", "bored"],
            &["r", "u", "bored"],
        ],
    ),
    (
        Question::Lonely,
        &[
            &["are", "you", "lonely"],
            &["did", "you", "miss", "me"],
            &["do", "you", "miss", "me"],
            &["you", "lonely"],
            &["miss", "me"],
        ],
    ),
    (
        Question::Doing,
        &[
            &["what", "are", "you", "doing"],
            &["what", "you", "doing"],
            &["whatcha", "doing"],
            &["whatcha", "doin"],
            &["what", "are", "you", "up", "to"],
            &["whats", "up"],
            &["wyd"],
            &["wassup"],
            &["sup"],
        ],
    ),
    (
        Question::Feeling,
        &[
            &["how", "are", "you"],
            &["how", "r", "u"],
            &["how", "do", "you", "feel"],
            &["how", "you", "doing"],
            &["how", "are", "things"],
            &["hows", "it", "going"],
            &["how", "have", "you", "been"],
            &["are", "you", "ok"],
            &["are", "you", "okay"],
            &["are", "you", "alright"],
            &["are", "you", "happy"],
            &["you", "ok"],
            &["you", "okay"],
        ],
    ),
];

// Questions in Korean, as pieces of the message with its spaces taken out,
// and whether the message must end in a question mark, for words that are
// statements just as often ("졸려" is also "I'm sleepy").
const KOREAN: &[(Question, &[&str], bool)] = &[
    (
        Question::Mirror,
        &["거울속에누구", "거울에누가있어", "거울속엔누가"],
        false,
    ),
    (Question::Music, &["음악좋아", "노래좋아"], false),
    (
        Question::Songs,
        &["무슨노래알아", "어떤노래알아", "아는노래"],
        false,
    ),
    (
        Question::Wearing,
        &["뭐입었어", "뭐입고있어", "그거뭐야"],
        true,
    ),
    (
        Question::ReadAloud,
        &["책읽어줘", "책읽어줄래", "책읽어주라"],
        false,
    ),
    (
        Question::Reading,
        &["무슨책읽", "어떤책읽", "뭐읽어"],
        false,
    ),
    (
        Question::FavoriteBook,
        &["제일좋아하는책", "가장좋아하는책"],
        false,
    ),
    (
        Question::Wish,
        &["소원이뭐", "소원있어", "소원은뭐", "무슨소원"],
        false,
    ),
    (
        Question::Story,
        &[
            "이야기해줘",
            "이야기들려줘",
            "얘기해줘",
            "옛날이야기",
            "동화들려줘",
        ],
        false,
    ),
    (Question::Story, &["이야기", "얘기", "동화"], true),
    (
        Question::Away,
        &["뭐하고있었", "없는동안뭐", "안온동안뭐"],
        false,
    ),
    (
        Question::Constellations,
        &[
            "별자리알아",
            "별자리알고",
            "별자리찾았",
            "무슨별자리",
            "별들이뭐",
            "아는별자리",
        ],
        false,
    ),
    (
        Question::Friend,
        &[
            "친구는누구",
            "친구이름",
            "친구어디",
            "친구있어",
            "친구는어때",
        ],
        false,
    ),
    (
        Question::Shelf,
        &["선반에뭐", "보물이뭐", "보물뭐", "선물뭐받"],
        false,
    ),
    (Question::Bird, &["새좋아", "새랑친구", "새는친구"], true),
    (
        Question::ViewerName,
        &[
            "내이름이뭐",
            "내이름뭐",
            "내이름알",
            "내이름기억",
            "내가누구",
        ],
        false,
    ),
    (
        Question::Name,
        &[
            "이름이뭐",
            "이름뭐",
            "이름이머",
            "누구야",
            "누구니",
            "누구세요",
        ],
        false,
    ),
    (Question::Age, &["몇살", "나이가어떻게"], false),
    (
        Question::Liking,
        &["나좋아해", "나사랑해", "우리친구"],
        true,
    ),
    (
        Question::Favorite,
        &["뭐좋아해", "뭘좋아해", "제일좋아하는"],
        false,
    ),
    (Question::Weather, &["날씨어때", "날씨는어때"], false),
    (Question::Weather, &["비와", "눈와"], true),
    (
        Question::Tired,
        &["졸려", "피곤해", "졸리니", "피곤하니"],
        true,
    ),
    (Question::Hungry, &["배고파", "배고프니"], true),
    (Question::Bored, &["심심해", "심심하니"], true),
    (Question::Lonely, &["외로워", "외롭니", "보고싶었"], true),
    (
        Question::Doing,
        &["뭐해", "뭐하니", "뭐하고있", "뭐하는중", "머해"],
        false,
    ),
    (
        Question::Feeling,
        &["기분어때", "기분은어때", "잘지냈", "어떻게지내"],
        false,
    ),
    (Question::Feeling, &["잘지내", "괜찮아"], true),
];

/// The question `text` asks, if it is one the ghost can answer by itself.
pub fn ask(text: &str) -> Option<Question> {
    let words: Vec<String> = text
        .to_lowercase()
        .replace(['\'', '’'], "")
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect();
    let english = ENGLISH.iter().find(|(_, runs)| {
        runs.iter().any(|run| {
            words.windows(run.len()).any(|window| {
                window
                    .iter()
                    .zip(run.iter())
                    .all(|(word, part)| word == part)
            })
        })
    });
    if let Some((question, _)) = english {
        return Some(*question);
    }
    let asked = text.trim_end().ends_with(['?', '？']);
    if asked && matches!(words.as_slice(), [word] if word == "story")
        || asked && matches!(words.as_slice(), [article, word] if article == "a" && word == "story")
    {
        return Some(Question::Story);
    }
    let joined: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    KOREAN
        .iter()
        .find(|(_, pieces, needs_mark)| {
            (asked || !needs_mark) && pieces.iter().any(|piece| joined.contains(piece))
        })
        .map(|(question, _, _)| *question)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_requests_use_word_runs_and_korean_pieces() {
        for text in [
            "tell me a story",
            "story please",
            "a story?",
            "story?",
            "bedtime story",
            "can you tell me a story",
            "tell a story",
            "Please, TELL me a story!",
            "이야기 해줘",
            "이야기해 줘",
            "이야기 들려줘",
            "얘기해줘",
            "옛날 이야기",
            "동화 들려줘",
            "이야기?",
            "동화？",
        ] {
            assert_eq!(ask(text), Some(Question::Story), "{text}");
        }
        for text in [
            "story",
            "a story",
            "history please",
            "storytime",
            "I read a story",
            "a storybook?",
            "tell me a store",
            "이야기",
            "얘기",
            "동화",
            "동화책을 읽었어",
        ] {
            assert_eq!(ask(text), None, "{text}");
        }
    }

    #[test]
    fn everyday_questions_in_english_are_recognized() {
        for (text, question) in [
            ("What did you do while I was away?", Question::Away),
            ("what were you doing?", Question::Away),
            ("How are you?", Question::Feeling),
            ("hi! how's it going", Question::Feeling),
            ("are you okay", Question::Feeling),
            ("What are you doing?", Question::Doing),
            ("whatcha doing", Question::Doing),
            ("what's up", Question::Doing),
            ("What's your name?", Question::Name),
            ("what's my name?", Question::ViewerName),
            ("do you know my name", Question::ViewerName),
            ("who are you", Question::Name),
            ("Do you like me?", Question::Liking),
            ("do you love me", Question::Liking),
            ("How old are you?", Question::Age),
            ("what's your favorite thing", Question::Favorite),
            ("what do you like to do", Question::Favorite),
            ("How's the weather?", Question::Weather),
            ("is it raining", Question::Weather),
            ("are you tired?", Question::Tired),
            ("you sleepy?", Question::Tired),
            ("Are you hungry?", Question::Hungry),
            ("are you bored", Question::Bored),
            ("did you miss me?", Question::Lonely),
            ("What's on your shelf?", Question::Shelf),
            ("do you like the bird?", Question::Bird),
            ("do you know any constellations?", Question::Constellations),
            ("what are those stars?", Question::Constellations),
        ] {
            assert_eq!(ask(text), Some(question), "{text}");
        }
    }

    #[test]
    fn everyday_questions_in_korean_are_recognized() {
        for (text, question) in [
            ("뭐 하고 있었어?", Question::Away),
            ("뭐해?", Question::Doing),
            ("지금 뭐 하고 있어", Question::Doing),
            ("기분 어때?", Question::Feeling),
            ("잘 지냈어?", Question::Feeling),
            ("괜찮아?", Question::Feeling),
            ("비 와?", Question::Weather),
            ("이름이 뭐야?", Question::Name),
            ("내 이름이 뭐야?", Question::ViewerName),
            ("내 이름 알아?", Question::ViewerName),
            ("너 누구야", Question::Name),
            ("몇 살이야?", Question::Age),
            ("나 좋아해?", Question::Liking),
            ("뭐 좋아해?", Question::Favorite),
            ("날씨 어때?", Question::Weather),
            ("졸려?", Question::Tired),
            ("배고파?", Question::Hungry),
            ("심심해?", Question::Bored),
            ("보고 싶었어?", Question::Lonely),
            ("선반에 뭐 있어?", Question::Shelf),
            ("별자리 알아?", Question::Constellations),
            ("저 새 좋아해?", Question::Bird),
        ] {
            assert_eq!(ask(text), Some(question), "{text}");
        }
    }

    #[test]
    fn statements_requests_and_lookalikes_are_not_questions() {
        for text in [
            "hello",
            "good job",
            "fetch the ball",
            "I'm tired",
            "졸려",
            "배고파",
            "사랑해",
            "괜찮아",
            "잘 지내",
            "비 와",
            "how about a dance",
            "show me what you can do",
            "you are so cute",
            "",
        ] {
            assert_eq!(ask(text), None, "{text}");
        }
    }

    #[test]
    fn particular_questions_win_over_general_ones() {
        assert_eq!(ask("are you tired? are you okay?"), Some(Question::Tired));
        assert_eq!(ask("how are you, what's your name"), Some(Question::Name));
    }
    #[test]
    fn questions_about_wishes_use_english_word_runs_and_korean_pieces() {
        for text in [
            "what do you wish for?",
            "do you have a wish?",
            "what's your wish?",
            "What is your wish?",
            "소원이 뭐야?",
            "소원 있어?",
            "소원은 뭐야?",
        ] {
            assert_eq!(ask(text), Some(Question::Wish), "{text}");
        }
        for text in ["wishful thinking", "I wish for rain", "내 소원은 무지개야"] {
            assert_ne!(ask(text), Some(Question::Wish), "{text}");
        }
    }
}
