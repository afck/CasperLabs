use super::alloc::collections::btree_map::BTreeMap;
use super::alloc::string::String;
use super::alloc::vec::Vec;
use super::bytesrepr::{Error, FromBytes, ToBytes};
use super::key::{Key, UREF_SIZE};

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Value {
    Int32(i32),
    ByteArray(Vec<u8>),
    ListInt32(Vec<i32>),
    String(String),
    ListString(Vec<String>),
    NamedKey(String, Key),
    Acct(Account),
    Contract {
        bytes: Vec<u8>,
        known_urefs: BTreeMap<String, Key>,
    },
}

const INT32_ID: u8 = 0;
const BYTEARRAY_ID: u8 = 1;
const LISTINT32_ID: u8 = 2;
const STRING_ID: u8 = 3;
const ACCT_ID: u8 = 4;
const CONTRACT_ID: u8 = 5;
const NAMEDKEY_ID: u8 = 6;
const LISTSTRING_ID: u8 = 7;

use self::Value::*;

impl ToBytes for Value {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            Int32(i) => {
                let mut result = Vec::with_capacity(5);
                result.push(INT32_ID);
                result.append(&mut i.to_bytes());
                result
            }
            ByteArray(arr) => {
                let mut result = Vec::with_capacity(5 + arr.len());
                result.push(BYTEARRAY_ID);
                result.append(&mut arr.to_bytes());
                result
            }
            ListInt32(arr) => {
                let mut result = Vec::with_capacity(5 + 4 * arr.len());
                result.push(LISTINT32_ID);
                result.append(&mut arr.to_bytes());
                result
            }
            String(s) => {
                let mut result = Vec::with_capacity(5 + s.len());
                result.push(STRING_ID);
                result.append(&mut s.to_bytes());
                result
            }
            Acct(a) => {
                let mut result = Vec::new();
                result.push(ACCT_ID);
                result.append(&mut a.to_bytes());
                result
            }
            Contract { bytes, known_urefs } => {
                let size: usize = 1 +              //size for ID
                    4 +                            //size for length of bytes
                    bytes.len() +                  //size for elements of bytes
                    4 +                            //size for length of known_urefs
                    UREF_SIZE * known_urefs.len(); //size for known_urefs elements

                let mut result = Vec::with_capacity(size);
                result.push(CONTRACT_ID);
                result.append(&mut bytes.to_bytes());
                result.append(&mut known_urefs.to_bytes());
                result
            }
            NamedKey(n, k) => {
                let size: usize = 1 + //size for ID
                  4 +                 //size for length of String
                  n.len() +           //size of String
                  UREF_SIZE; //size of urefs
                let mut result = Vec::with_capacity(size);
                result.push(NAMEDKEY_ID);
                result.append(&mut n.to_bytes());
                result.append(&mut k.to_bytes());
                result
            }
            ListString(arr) => {
                let mut result = Vec::with_capacity(5 + arr.len());
                result.push(LISTSTRING_ID);
                result.append(&mut arr.to_bytes());
                result
            }
        }
    }
}
impl FromBytes for Value {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (id, rest): (u8, &[u8]) = FromBytes::from_bytes(bytes)?;
        match id {
            INT32_ID => {
                let (i, rem): (i32, &[u8]) = FromBytes::from_bytes(rest)?;
                Ok((Int32(i), rem))
            }
            BYTEARRAY_ID => {
                let (arr, rem): (Vec<u8>, &[u8]) = FromBytes::from_bytes(rest)?;
                Ok((ByteArray(arr), rem))
            }
            LISTINT32_ID => {
                let (arr, rem): (Vec<i32>, &[u8]) = FromBytes::from_bytes(rest)?;
                Ok((ListInt32(arr), rem))
            }
            STRING_ID => {
                let (s, rem): (String, &[u8]) = FromBytes::from_bytes(rest)?;
                Ok((String(s), rem))
            }
            ACCT_ID => {
                let (a, rem): (Account, &[u8]) = FromBytes::from_bytes(rest)?;
                Ok((Acct(a), rem))
            }
            CONTRACT_ID => {
                let (bytes, rem1): (Vec<u8>, &[u8]) = FromBytes::from_bytes(rest)?;
                let (known_urefs, rem2): (BTreeMap<String, Key>, &[u8]) =
                    FromBytes::from_bytes(rem1)?;
                Ok((Contract { bytes, known_urefs }, rem2))
            }
            NAMEDKEY_ID => {
                let (name, rem1): (String, &[u8]) = FromBytes::from_bytes(rest)?;
                let (key, rem2): (Key, &[u8]) = FromBytes::from_bytes(rem1)?;
                Ok((NamedKey(name, key), rem2))
            }
            LISTSTRING_ID => {
                let (arr, rem): (Vec<String>, &[u8]) = FromBytes::from_bytes(rest)?;
                Ok((ListString(arr), rem))
            }
            _ => Err(Error::FormattingError),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Account {
    public_key: [u8; 32],
    nonce: u64,
    known_urefs: BTreeMap<String, Key>,
}

impl ToBytes for Account {
    fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend(&self.public_key.to_bytes());
        result.append(&mut self.nonce.to_bytes());
        result.append(&mut self.known_urefs.to_bytes());
        result
    }
}
impl FromBytes for Account {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (public_key, rem1): ([u8; 32], &[u8]) = FromBytes::from_bytes(bytes)?;
        let (nonce, rem2): (u64, &[u8]) = FromBytes::from_bytes(rem1)?;
        let (known_urefs, rem3): (BTreeMap<String, Key>, &[u8]) = FromBytes::from_bytes(rem2)?;
        Ok((
            Account {
                public_key,
                nonce,
                known_urefs,
            },
            rem3,
        ))
    }
}

impl Value {
    pub fn type_string(&self) -> String {
        match self {
            Int32(_) => String::from("Int32"),
            ListInt32(_) => String::from("List[Int32]"),
            String(_) => String::from("String"),
            ByteArray(_) => String::from("ByteArray"),
            Acct(_) => String::from("Account"),
            Contract { .. } => String::from("Contract"),
            NamedKey(_, _) => String::from("NamedKey"),
            ListString(_) => String::from("List[String]"),
        }
    }

    pub fn as_account(&self) -> &Account {
        match self {
            Acct(a) => a,
            _ => panic!("Not an account: {:?}", self),
        }
    }
}

impl Account {
    pub fn new(public_key: [u8; 32], nonce: u64, known_urefs: BTreeMap<String, Key>) -> Account {
        Account {
            public_key,
            nonce,
            known_urefs,
        }
    }

    pub fn insert_urefs(&mut self, keys: &mut BTreeMap<String, Key>) {
        self.known_urefs.append(keys);
    }

    pub fn urefs_lookup(&self) -> &BTreeMap<String, Key> {
        &self.known_urefs
    }

    pub fn get_urefs_lookup(self) -> BTreeMap<String, Key> {
        self.known_urefs
    }

    pub fn pub_key(&self) -> &[u8] {
        &self.public_key
    }

    pub fn nonce(&self) -> u64 {
        self.nonce
    }
}
