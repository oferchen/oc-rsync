// crates/protocol/src/types/charset.rs
use encoding_rs::Encoding;
use std::borrow::Cow;

#[derive(Clone)]
pub struct CharsetConv {
    remote: &'static Encoding,
    local: &'static Encoding,
}

impl CharsetConv {
    pub fn encode_remote<'a>(&self, s: &'a str) -> Cow<'a, [u8]> {
        let (res, _, _) = self.remote.encode(s);
        res
    }

    pub fn decode_remote<'a>(&self, b: &'a [u8]) -> Cow<'a, str> {
        let (res, _, _) = self.remote.decode(b);
        res
    }

    pub fn to_remote<'a>(&self, b: &'a [u8]) -> Cow<'a, [u8]> {
        if self.remote == self.local {
            Cow::Borrowed(b)
        } else {
            let (s, _, _) = self.local.decode(b);
            Cow::Owned(self.remote.encode(&s).0.into_owned())
        }
    }

    pub fn to_local<'a>(&self, b: &'a [u8]) -> Cow<'a, [u8]> {
        if self.remote == self.local {
            Cow::Borrowed(b)
        } else {
            let (s, _, _) = self.remote.decode(b);
            Cow::Owned(self.local.encode(&s).0.into_owned())
        }
    }

    pub fn new(remote: &'static Encoding, local: &'static Encoding) -> Self {
        Self { remote, local }
    }
}
