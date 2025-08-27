use chumsky::{extra::ParserExtra, prelude::*};
use itertools::{Itertools, chain};

pub const ENDPOINT: (&str, u16) = ("eaccess.play.net", 7900);

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    ParseError(String),
}

/// Hashes a password using the hash key provided by play.net
pub fn hash_password(
    password: impl Iterator<Item = u8>,
    hash_key: impl Iterator<Item = u8>,
) -> impl Iterator<Item = u8> {
    password.zip(hash_key).map(|(p, h)| ((p - 0x20) ^ h) + 0x20)
}

#[derive(Debug, Clone)]
pub struct K<'a> {
    pub key: &'a str,
}

impl<'a> K<'a> {
    #[inline(always)]
    pub const fn out() -> &'static str {
        "K\n"
    }
}

impl<'a> Message<'a> for K<'a> {
    fn parser() -> impl Parser<'a, &'a str, Self, extra::Err<Simple<'a, char>>> {
        taken_ignore(just('\n')).map(|key| Self { key })
    }
}

#[derive(Debug, Clone)]
pub struct A<'a> {
    pub account: &'a str,
    pub key: &'a str,
    pub name: &'a str,
}

impl A<'_> {
    pub fn out(
        account: impl Iterator<Item = u8>,
        hashed_password: impl Iterator<Item = u8>,
    ) -> Vec<u8> {
        chain!([b'A', b'\t'], account, [b'\t'], hashed_password, [b'\n']).collect()
    }
}

impl<'a> Message<'a> for A<'a> {
    fn parser() -> impl Parser<'a, &'a str, Self, extra::Err<Simple<'a, char>>> {
        group((
            just("A\t").ignore_then(taken_ignore(just('\t'))),
            just("KEY\t").ignore_then(taken_ignore(just('\t'))),
            taken_ignore(just('\n')),
        ))
        .map(|(account, key, name)| Self { account, key, name })
    }
}

#[derive(Debug, Clone)]
pub struct M<'a>(pub Vec<(&'a str, &'a str)>);

impl M<'_> {
    #[inline(always)]
    pub const fn out() -> &'static str {
        "M\n"
    }
}

impl<'a> Message<'a> for M<'a> {
    fn parser() -> impl Parser<'a, &'a str, Self, extra::Err<Simple<'a, char>>> {
        just("M\t")
            .ignore_then(
                group((taken_ignore(just('\t')), taken_ignore(one_of("\t\n"))))
                    .repeated()
                    .at_least(1)
                    .collect(),
            )
            .map(|v: Vec<(&'a str, &'a str)>| Self(v))
    }
}
#[derive(Debug, Clone)]
pub enum NEnvironment<'a> {
    /// PRODUCTION
    Production,
    /// DEVELOPMENT
    Development,
    Other(&'a str),
}

impl<'a> From<&'a str> for NEnvironment<'a> {
    fn from(value: &'a str) -> Self {
        match value {
            "PRODUCTION" => Self::Production,
            "DEVELOPMENT" => Self::Development,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NProtocol<'a> {
    /// STORM
    Storm,
    Other(&'a str),
}

impl<'a> From<NProtocol<'a>> for &'a str {
    fn from(value: NProtocol<'a>) -> Self {
        match value {
            NProtocol::Storm => "STORM",
            NProtocol::Other(other) => other,
        }
    }
}

impl<'a> From<&'a str> for NProtocol<'a> {
    fn from(value: &'a str) -> Self {
        match value {
            "STORM" => Self::Storm,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NAccess<'a> {
    /// no value
    None,
    /// TRIAL
    Trial,
    Other(&'a str),
}

impl<'a> From<Option<&'a str>> for NAccess<'a> {
    fn from(value: Option<&'a str>) -> Self {
        match value {
            None => Self::None,
            Some("TRIAL") => Self::Trial,
            Some(other) => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct N<'a> {
    pub environment: NEnvironment<'a>,
    pub protocol: NProtocol<'a>,
    pub access: NAccess<'a>,
}

impl N<'_> {
    pub fn out(node: &str) -> String {
        format!("N\t{node}\n")
    }
}

impl<'a> Message<'a> for N<'a> {
    fn parser() -> impl Parser<'a, &'a str, Self, extra::Err<Simple<'a, char>>> {
        group((
            just("N\t").ignore_then(taken_ignore(just('|')).map(|v| v.into())),
            taken_ignore(one_of("|\n")).map(|v| v.into()),
            taken_ignore(just('\n')).or_not().map(|v| v.into()),
        ))
        .map(|(environment, protocol, access)| Self {
            environment,
            protocol,
            access,
        })
    }
}

#[derive(Debug, Clone)]
pub enum PaymentStatus<'a> {
    /// NEED_BILL
    NeedBill,
    /// FREE
    Free,
    /// FREE_TO_PLAY
    FreeToPlay,
    /// EXPIRED (https://gswiki.play.net/SGE_protocol/saved_posts)
    ///
    /// I have not seen this status code personally.
    Expired,
    /// NEW_TO_GAME (https://gswiki.play.net/SGE_protocol/saved_posts)
    ///
    /// I have not seen this status code personally.
    NewToGame,
    Other(&'a str),
}

impl<'a> From<&'a str> for PaymentStatus<'a> {
    fn from(value: &'a str) -> Self {
        match value {
            "NEED_BILL" => Self::NeedBill,
            "FREE" => Self::Free,
            "FREE_TO_PLAY" => Self::FreeToPlay,
            "EXPIRED" => Self::Expired,
            "NEW_TO_GAME" => Self::NewToGame,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct F<'a>(pub PaymentStatus<'a>);

impl F<'_> {
    pub fn out(node: &str) -> String {
        format!("F\t{node}\n")
    }
}

impl<'a> Message<'a> for F<'a> {
    fn parser() -> impl Parser<'a, &'a str, Self, extra::Err<Simple<'a, char>>> {
        just("F\t")
            .ignore_then(taken_ignore(just('\n')))
            .map(|v| Self(v.into()))
    }
}

/// This struct requests general info for an instance, and includes links.
///
/// Send this struct before sending C (character request)
#[derive(Debug, Clone)]
pub struct G<'a> {
    pub name: &'a str,
    pub model: PaymentStatus<'a>,
    pub data: Vec<(&'a str, &'a str)>,
}

impl G<'_> {
    pub fn out(node: &str) -> String {
        format!("G\t{node}\n")
    }
}

impl<'a> Message<'a> for G<'a> {
    fn parser() -> impl Parser<'a, &'a str, Self, extra::Err<Simple<'a, char>>> {
        group((
            just("G\t").ignore_then(taken_ignore(just('\t'))),
            taken_ignore(just('\t')).then_ignore(just("0\t\t")),
            group((taken_ignore(just('=')), taken_ignore(one_of("\t\n"))))
                .repeated()
                .at_least(1)
                .collect(),
        ))
        .map(|(name, model, data)| Self {
            name,
            model: model.into(),
            data,
        })
    }
}

/// I have no idea what this information means or what the message does.
#[derive(Debug, Clone)]
pub struct P<'a> {
    pub p0: &'a str,
    pub p1: &'a str,
    pub p2: &'a str,
    pub p3: &'a str,
    pub p4: &'a str,
    pub p5: &'a str,
}

impl P<'_> {
    pub fn out(node: &str) -> String {
        format!("P\t{node}\n")
    }
}

impl<'a> Message<'a> for P<'a> {
    fn parser() -> impl Parser<'a, &'a str, Self, extra::Err<Simple<'a, char>>> {
        group((
            just("P\t").ignore_then(taken_ignore(just('\t'))),
            taken_ignore(just('\t')),
            taken_ignore(just('\t')),
            taken_ignore(just('\t')),
            taken_ignore(just('\t')),
            taken_ignore(just('\n')),
        ))
        .map(|(p0, p1, p2, p3, p4, p5)| Self {
            p0,
            p1,
            p2,
            p3,
            p4,
            p5,
        })
    }
}

/// Send/parse request for character list for a specific instance.
///
/// Note that you cannot include an instance ID in the request; instead, you must send
/// `G::out(<node>)` first, after which `C::out` will return the characters for the node.
#[derive(Debug, Clone)]
pub struct C<'a> {
    pub num_characters: u64,
    pub max_characters: u64,
    /// no clue what this number is
    pub n0: u64,
    /// no clue what this number is
    pub n1: u64,
    pub characters: Vec<(&'a str, &'a str)>,
}

impl C<'_> {
    #[inline(always)]
    pub const fn out() -> &'static str {
        "C\n"
    }
}

impl<'a> Message<'a> for C<'a> {
    fn parser() -> impl Parser<'a, &'a str, Self, extra::Err<Simple<'a, char>>> {
        group((
            just("C\t").ignore_then(number().then_ignore(just('\t'))),
            number().then_ignore(just('\t')),
            number().then_ignore(just('\t')),
            number().then_ignore(one_of("\t\n")),
            taken_ignore(just('\t'))
                .then(taken_ignore(one_of("\t\n")))
                .repeated()
                .collect(),
        ))
        .map(|(nc, ns, n0, n1, characters)| Self {
            num_characters: nc,
            max_characters: ns,
            n0,
            n1,
            characters,
        })
    }
}

/// You likely want (`game_host`:`game_port`) and `key`
#[derive(Debug, Clone)]
pub struct L<'a> {
    /// UPPORT
    pub upport: u64,
    /// GAME
    pub game: &'a str,
    /// GAMECODE
    pub game_code: &'a str,
    /// FULLGAMENAME
    pub full_game_name: &'a str,
    /// GAMEFILE
    pub game_file: &'a str,
    /// GAMEHOST
    pub game_host: &'a str,
    /// GAMEPORT
    pub game_port: u64,
    /// KEY
    pub key: &'a str,
}

impl L<'_> {
    pub fn out<'a>(character_id: &str, protocol: impl Into<&'a str>) -> String {
        format!("L\t{character_id}\t{}\n", Into::<&str>::into(protocol))
    }
}

impl<'a> Message<'a> for L<'a> {
    fn parser() -> impl Parser<'a, &'a str, Self, extra::Err<Simple<'a, char>>> {
        group((
            just("L\tOK\tUPPORT=").ignore_then(number().then_ignore(just('\t'))),
            just("GAME=").ignore_then(taken_ignore(just('\t'))),
            just("GAMECODE=").ignore_then(taken_ignore(just('\t'))),
            just("FULLGAMENAME=").ignore_then(taken_ignore(just('\t'))),
            just("GAMEFILE=").ignore_then(taken_ignore(just('\t'))),
            just("GAMEHOST=").ignore_then(taken_ignore(just('\t'))),
            just("GAMEPORT=").ignore_then(number().then_ignore(just('\t'))),
            just("KEY=").ignore_then(taken_ignore(just('\n'))),
        ))
        .map(
            |(upport, game, game_code, full_game_name, game_file, game_host, game_port, key)| {
                Self {
                    upport,
                    game,
                    game_code,
                    full_game_name,
                    game_file,
                    game_host,
                    game_port,
                    key,
                }
            },
        )
    }
}

pub trait Message<'a>: Sized {
    fn parser() -> impl Parser<'a, &'a str, Self, extra::Err<Simple<'a, char>>>;

    fn parse(inp: &'a str) -> Result<Self, Error> {
        let res = Self::parser().then_ignore(end()).parse(inp);

        res.into_result()
            .map_err(|e| Error::ParseError(e.into_iter().map(|e| e.to_string()).join("\n")))
    }
}

fn number<'a>() -> impl Parser<'a, &'a str, u64, extra::Err<Simple<'a, char>>> {
    one_of("0123456789")
        .repeated()
        .to_slice()
        .map(|v: &str| v.parse().unwrap())
}

fn taken_ignore<'a, O, E>(
    parser: impl Parser<'a, &'a str, O, E> + Clone,
) -> impl Parser<'a, &'a str, &'a str, E>
where
    E: ParserExtra<'a, &'a str>,
{
    any()
        .and_is(parser.clone().not())
        .repeated()
        .to_slice()
        .then_ignore(parser)
}
