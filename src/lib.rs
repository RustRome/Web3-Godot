extern crate ethabi;
extern crate gdnative;
extern crate web3;

use ethabi::{ParamType, Token};
use gdnative::*;
use std::iter::IntoIterator;
use web3::contract::{tokens::Tokenizable, Contract, Error, Options, QueryResult};
use web3::futures::Future;
use web3::types::{Address, U256, H160};

struct VariantArray(gdnative::VariantArray);

impl VariantArray {
    fn new(variant: gdnative::VariantArray) -> Self {
        VariantArray(variant)
    }

    fn to_vec(&self) -> Vec<Token> {
        self.0
            .iter()
            .map(|s| {
                if let Some(string) = s.try_to_string() {
                    if string.starts_with("0x") {
                        Token::Address(string.as_str().parse().unwrap())
                    }else{
                        Token::String(string)
                    }
                } else if let Some(number) = s.try_to_i64() {
                    Token::Uint(U256::from(number as u64))
                } else if let Some(boolean) = s.try_to_bool() {
                    Token::Bool(boolean)
                } else if let Some(array) = s.try_to_array() {
                    Token::Array(VariantArray(array).to_vec())
                } else{
                    Token::Uint(U256::from(0))
                }
            })
            .filter(|t| !t.type_check(&ParamType::Bool))
            .collect::<Vec<Token>>()
    }
}

impl VariantArray {
    fn from_vector(vector: Vec<Token>) -> Result<Self, Error> {
        let mut variant_array: VariantArray = VariantArray::new(gdnative::VariantArray::new());
        for token_element in vector {
            match token_element {
                Token::Uint(number) | Token::Int(number) => {
                    variant_array
                        .0
                        .push(&gdnative::Variant::from_u64(number.as_u64()));
                }
                Token::String(string) => {
                    variant_array
                        .0
                        .push(&gdnative::Variant::from_str(string.as_str()));
                }
                Token::Bool(boolean) => {
                    variant_array
                        .0
                        .push(&gdnative::Variant::from_bool(boolean));
                }
                Token::Bytes(bytes) => {
                    let mut tmp_bytearray : ByteArray = ByteArray::new();
                    bytes.iter().map(|s| tmp_bytearray.push(*s));
                    variant_array
                        .0
                        .push(&gdnative::Variant::from_byte_array(&tmp_bytearray));
                }
                Token::FixedArray(tokens) | Token::Array(tokens) => {
                    variant_array
                        .0
                        .push(&gdnative::Variant::from_array(&VariantArray::from_vector(tokens).unwrap().0));
                }
                _ => return Err(Error::InvalidOutputType("Error".to_owned())),
            }
        }
        return Ok(variant_array);
    }
}

impl Tokenizable for VariantArray {
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::FixedArray(tokens) | Token::Array(tokens) => {
                return VariantArray::from_vector(tokens);
            }
            other => Err(Error::InvalidOutputType(format!(
                "Expected `Array`, got {:?}",
                other
            ))),
        }
    }

    fn into_token(self) -> Token {
        Token::Array(self.to_vec())
    }
}

#[derive(NativeClass)]
#[inherit(Node)]
pub struct Web3Godot {
    web3: Option<web3::Web3<web3::transports::Http>>,
    contract: Option<web3::contract::Contract<web3::transports::Http>>,
}

#[methods]
impl Web3Godot {
    fn _init(_owner: Node) -> Self {
        Web3Godot {
            web3: None,
            contract: None,
        }
    }

    #[export]
    fn initialize_http_transport(&mut self, _owner: gdnative::Node, http_url: String) {
        let (eloop, http) = web3::transports::Http::new(http_url.as_str()).unwrap();
        eloop.into_remote();
        self.web3 = Some(web3::Web3::new(http));
    }

    #[export]
    fn initialize_smart_contract( &mut self, _owner: gdnative::Node, abi_file: String, sc_address: String ) {
        self.contract = Some(
            web3::contract::Contract::from_json(
                self.web3.as_ref().unwrap().eth(),
                sc_address.as_str().parse().unwrap(),
                std::fs::read(abi_file.as_str()).unwrap().as_ref(),
            ).unwrap(),
        );
    }

    #[export]
    fn call( &self, _owner: gdnative::Node, function_name: String,
             parameters: gdnative::VariantArray, from: String ) {
        let tmp_tokenized_parameters = VariantArray( parameters );
        self.contract.as_ref().unwrap().call( function_name.as_str(),
                                              tmp_tokenized_parameters.to_vec().as_slice(),
                                              from.parse().unwrap(), Options::default() );
    }

    #[export]
    fn query( &self, _owner: gdnative::Node, function_name: String,
              parameters: gdnative::VariantArray, from: String ) {
        let tmp_tokenized_parameters = VariantArray( parameters );
        let _: QueryResult<VariantArray, _> = self.contract.as_ref().unwrap().query( function_name.as_str(),
                                              tmp_tokenized_parameters.to_vec().as_slice(),
                                              Some(H160::from_slice(from.as_str().as_bytes())), Options::default(), None );
    }
}

fn init(handle: gdnative::init::InitHandle) {
    handle.add_class::<Web3Godot>();
}

godot_gdnative_init!();
godot_nativescript_init!(init);
godot_gdnative_terminate!();
