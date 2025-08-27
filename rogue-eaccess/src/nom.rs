use itertools::chain;
use nom::{
    Finish, IResult, Parser,
    bytes::complete::{is_not, tag, take_until, take_while},
    character::complete::one_of,
    combinator::{all_consuming, opt},
    error::{Error as NomError, ParseError as NomParseError},
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, separated_pair, terminated},
};
use std::fmt::Debug;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("test")]
    ParseError(#[from] NomError<String>),
}

pub const ENDPOINT: (&str, u16) = ("eaccess.play.net", 7900);

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

impl K<'_> {
    #[inline(always)]
    pub const fn out() -> &'static str {
        "K\n"
    }
}

impl<'a> Message<'a> for K<'a> {
    fn parse_raw<E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Self, E> {
        let (i, key) = word(i)?;

        Ok((i, Self { key }))
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
    fn parse_raw<E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Self, E> {
        let (i, (account, key, name)) = (
            preceded(tag("A\t"), word),
            preceded(tag("KEY\t"), word),
            word,
        )
            .parse(i)?;

        Ok((i, Self { account, key, name }))
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
    fn parse_raw<E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Self, E> {
        let (i, val) = preceded(tag("M\t"), many1(pair(word, word))).parse(i)?;

        Ok((i, Self(val)))
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
    fn parse_raw<E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Self, E> {
        let (i, (environment, protocol, access)) = (
            delimited(tag("N\t"), take_until("|"), tag("|")),
            terminated(is_not("|\n"), one_of("|\n")),
            opt(terminated(take_until("\n"), tag("\n"))),
        )
            .parse(i)?;

        Ok((
            i,
            Self {
                environment: environment.into(),
                protocol: protocol.into(),
                access: access.into(),
            },
        ))
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
    fn parse_raw<E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Self, E> {
        let (i, val) = delimited(tag("F\t"), take_until("\n"), tag("\n")).parse(i)?;

        Ok((i, Self(val.into())))
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
    fn parse_raw<E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Self, E> {
        let (i, (name, model, data)) = (
            preceded(tag("G\t"), word),
            word,
            preceded(
                tag("0\t\t"),
                many1(terminated(
                    separated_pair(take_until("="), tag("="), is_not("\t\n")),
                    one_of("\t\n"),
                )),
            ),
        )
            .parse(i)?;

        Ok((
            i,
            Self {
                name,
                model: model.into(),
                data,
            },
        ))
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
    fn parse_raw<E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Self, E> {
        let (i, (p0, p1, p2, p3, p4, p5)) =
            (preceded(tag("P\t"), word), word, word, word, word, word).parse(i)?;

        Ok((
            i,
            Self {
                p0,
                p1,
                p2,
                p3,
                p4,
                p5,
            },
        ))
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
    fn parse_raw<E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Self, E> {
        let (i, (nc, ns, n0, n1, characters)) = (
            delimited(tag("C\t"), number, tag("\t")),
            terminated(number, tag("\t")),
            terminated(number, tag("\t")),
            terminated(number, one_of("\t\n")),
            many0(pair(word, word)),
        )
            .parse(i)?;

        Ok((
            i,
            Self {
                num_characters: nc,
                max_characters: ns,
                n0,
                n1,
                characters,
            },
        ))
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
    fn parse_raw<E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Self, E> {
        let (i, (upport, game, game_code, full_game_name, game_file, game_host, game_port, key)) =
            (
                delimited(tag("L\tOK\tUPPORT="), number, tag("\t")),
                delimited(tag("GAME="), take_until("\t"), tag("\t")),
                delimited(tag("GAMECODE="), take_until("\t"), tag("\t")),
                delimited(tag("FULLGAMENAME="), take_until("\t"), tag("\t")),
                delimited(tag("GAMEFILE="), take_until("\t"), tag("\t")),
                delimited(tag("GAMEHOST="), take_until("\t"), tag("\t")),
                delimited(tag("GAMEPORT="), number, tag("\t")),
                delimited(tag("KEY="), take_until("\n"), tag("\n")),
            )
                .parse(i)?;

        Ok((
            i,
            Self {
                upport,
                game,
                game_code,
                full_game_name,
                game_file,
                game_host,
                game_port,
                key,
            },
        ))
    }
}

pub trait Message<'a>: Sized {
    fn parse_raw<E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Self, E>;

    fn parse(i: &'a str) -> Result<Self, Error> {
        all_consuming(Self::parse_raw::<NomError<&str>>)
            .parse(i)
            .finish()
            .map(|v| v.1)
            .map_err(|e| e.cloned().into())
    }
}

fn word<'a, E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let (i, val) = terminated(is_not("\t\n"), one_of("\t\n")).parse(i)?;

    Ok((i, val))
}

fn number<'a, E: NomParseError<&'a str>>(i: &'a str) -> IResult<&'a str, u64, E> {
    let (i, val) = take_while(|c: char| c.is_ascii_digit()).parse(i)?;

    Ok((i, val.parse().unwrap()))
}
