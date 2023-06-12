use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

pub static SUBJECT_REPLACER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?P<title>\{title\})").expect("failed to compile subject replacer regex")
});

pub fn format_subject_title<'a>(subject: &'a str, title: &str) -> Cow<'a, str> {
    let Some(captures) = SUBJECT_REPLACER.captures(subject)
    else {
        return Cow::Borrowed(subject)
    };

    let Some(title_match) = captures.name("title")
    else {
        return Cow::Borrowed(subject);
    };

    let mut new_subject = String::new();
    new_subject.push_str(&subject[..title_match.start()]);
    new_subject.push_str(title);
    new_subject.push_str(&subject[title_match.end()..]);

    Cow::Owned(new_subject)
}
