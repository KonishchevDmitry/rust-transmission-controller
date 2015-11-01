use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, BufReader, BufRead};
use std::path::Path;

use regex::Regex;
use libemail::Mailbox;

use lettre::email::EmailBuilder;
use lettre::mailer::Mailer as LettreMailer;
use lettre::transport::smtp::SmtpTransportBuilder;

use common::GenericResult;

#[derive(Debug)]
pub struct Mailer {
    from: Mailbox,
    to: Mailbox,
}

#[derive(Debug)]
pub struct EmailTemplate {
    subject: String,
    body: String,
}

impl Mailer {
    pub fn new(from: &str, to: &str) -> GenericResult<Mailer> {
        Ok(Mailer {
            from: try!(parse_email_address(from)),
            to: try!(parse_email_address(to)),
        })
    }

    pub fn send(&self, subject: &str, body: &str) -> GenericResult<()> {
        let email = try!(EmailBuilder::new()
            .to(self.to.clone())
            .from(self.from.clone())
            .subject(subject)
            .body(body)
            .build());

        let transport = try!(SmtpTransportBuilder::localhost()).build();

        try!(LettreMailer::new(transport).send(email));

        Ok(())
    }
}

impl EmailTemplate {
    pub fn new(subject: &str, body: &str) -> EmailTemplate {
        EmailTemplate {
            subject: s!(subject),
            body: s!(body),
        }
    }

    pub fn new_from_file<P: AsRef<Path>>(path: P) -> GenericResult<EmailTemplate> {
        let mut file = BufReader::new(try!(File::open(path)));

        let mut subject = String::new();
        try!(file.read_line(&mut subject));

        let subject = subject.trim();
        if subject.is_empty() {
            return Err!("The first line must be a message Subject")
        }

        let mut delimeter = String::new();
        try!(file.read_line(&mut delimeter));
        if !delimeter.trim_right_matches(|c| c == '\r' || c == '\n').is_empty() {
            return Err!("The second line must be empty")
        }

        let mut body = String::new();
        try!(file.read_to_string(&mut body));

        Ok(EmailTemplate::new(subject, &body))
    }

    pub fn send(&self, mailer: &Mailer, params: &HashMap<&str, String>) -> GenericResult<()> {
        let (subject, body) = try!(self.render(&params));
        Ok(try!(mailer.send(&subject, &body)))
    }

    pub fn render(&self, params: &HashMap<&str, String>) -> GenericResult<(String, String)> {
        Ok((
            try!(render_template(&self.subject, params)),
            try!(render_template(&self.body, params)),
        ))
    }
}

fn parse_email_address(email: &str) -> GenericResult<Mailbox> {
    let email_address_re = r"(?P<address>[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+)";
    let email_re = Regex::new(&(s!("^") + email_address_re + "$")).unwrap();
    let email_with_name_re = Regex::new(&(s!(r"(?P<name>[^<]+)<") + email_address_re + ">$")).unwrap();

    Ok(match email_with_name_re.captures(email.trim()) {
        Some(captures) => Mailbox::new_with_name(
            s!(captures.name("name").unwrap().trim()), s!(captures.name("address").unwrap())),

        None => match email_re.captures(email) {
            Some(captures) => Mailbox::new(s!(captures.name("address").unwrap())),
            None => return Err!("Invalid email: '{}'", email)
        }
    })
}

fn render_template(template: &str, params: &HashMap<&str, String>) -> GenericResult<String> {
    // FIXME: Use very naive implementation now because Rust doesn't have any mature template engine yet.
    let mut result = s!(template);

    for (key, value) in params {
        let key = s!("{{") + key + "}}";
        result = result.replace(&key, &value);
    }

    Ok(result)
}