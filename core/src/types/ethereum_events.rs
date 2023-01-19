//! Types representing data intended for Namada via Ethereum events

use std::fmt::Display;
use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use ethabi::{Token, Uint as ethUint};
use eyre::{eyre, Context};
use serde::{Deserialize, Serialize};

use crate::types::address::Address;
use crate::types::eth_abi::Encode;
use crate::types::hash::Hash;
use crate::types::keccak::KeccakHash;
use crate::types::storage::{DbKeySeg, KeySeg};
use crate::types::token::Amount;

/// Namada native type to replace the ethabi::Uint type
#[derive(
    Clone,
    Debug,
    Default,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    BorshSchema,
)]
pub struct Uint(pub [u64; 4]);

impl Uint {
    /// Convert to a little endian byte representation of
    /// a uint256.
    pub fn to_bytes(self) -> [u8; 32] {
        let mut bytes = [0; 32];
        ethUint::from(self).to_little_endian(&mut bytes);
        bytes
    }
}

impl Encode<1> for Uint {
    fn tokenize(&self) -> [Token; 1] {
        [Token::Uint(self.into())]
    }
}

impl From<ethUint> for Uint {
    fn from(value: ethUint) -> Self {
        Self(value.0)
    }
}

impl From<Uint> for ethUint {
    fn from(value: Uint) -> Self {
        Self(value.0)
    }
}

impl From<&Uint> for ethUint {
    fn from(value: &Uint) -> Self {
        Self(value.0)
    }
}

impl From<u64> for Uint {
    fn from(value: u64) -> Self {
        ethUint::from(value).into()
    }
}

/// Representation of address on Ethereum. The inner value is the last 20 bytes
/// of the public key that controls the account.
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    BorshSchema,
)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub struct EthAddress(pub [u8; 20]);

impl EthAddress {
    /// The canonical way we represent an [`EthAddress`] in storage keys. A
    /// 40-character lower case hexadecimal address prefixed by '0x'.
    /// e.g. "0x6b175474e89094c44da98b954eedeac495271d0f"
    pub fn to_canonical(&self) -> String {
        format!("{:?}", ethabi::ethereum_types::Address::from(&self.0))
    }
}

impl Display for EthAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_canonical())
    }
}

impl FromStr for EthAddress {
    type Err = eyre::Error;

    /// Parses an [`EthAddress`] from a standard hex-encoded Ethereum address
    /// string. e.g. "0x6B175474E89094C44Da98b954EedeAC495271d0F"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let h160 = ethabi::ethereum_types::Address::from_str(s)
            .wrap_err_with(|| eyre!("couldn't parse Ethereum address {}", s))?;
        Ok(Self(h160.into()))
    }
}

impl TryFrom<String> for EthAddress {
    type Error = eyre::Error;

    fn try_from(string: String) -> Result<Self, eyre::Error> {
        Self::from_str(string.as_ref())
    }
}

impl From<EthAddress> for String {
    fn from(addr: EthAddress) -> Self {
        addr.to_string()
    }
}

impl KeySeg for EthAddress {
    fn parse(string: String) -> crate::types::storage::Result<Self> {
        Self::from_str(string.as_str())
            .map_err(|_| crate::types::storage::Error::ParseKeySeg(string))
    }

    fn raw(&self) -> String {
        self.to_canonical()
    }

    fn to_db_key(&self) -> DbKeySeg {
        DbKeySeg::StringSeg(self.raw())
    }
}

/// An Ethereum event to be processed by the Namada ledger
#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Hash,
    Ord,
    Clone,
    Debug,
    BorshSerialize,
    BorshDeserialize,
    BorshSchema,
)]
pub enum EthereumEvent {
    /// Event transferring batches of ether or Ethereum based ERC20 tokens
    /// from Ethereum to wrapped assets on Namada
    TransfersToNamada {
        /// Monotonically increasing nonce
        #[allow(dead_code)]
        nonce: Uint,
        /// The batch of transfers
        #[allow(dead_code)]
        transfers: Vec<TransferToNamada>,
    },
    /// A confirmation event that a batch of transfers have been made
    /// from Namada to Ethereum
    TransfersToEthereum {
        /// Monotonically increasing nonce
        #[allow(dead_code)]
        nonce: Uint,
        /// The batch of transfers
        #[allow(dead_code)]
        transfers: Vec<TransferToEthereum>,
    },
    /// Event indication that the validator set has been updated
    /// in the governance contract
    ValidatorSetUpdate {
        /// Monotonically increasing nonce
        #[allow(dead_code)]
        nonce: Uint,
        /// Hash of the validators in the bridge contract
        #[allow(dead_code)]
        bridge_validator_hash: KeccakHash,
        /// Hash of the validators in the governance contract
        #[allow(dead_code)]
        governance_validator_hash: KeccakHash,
    },
    /// Event indication that a new smart contract has been
    /// deployed
    NewContract {
        /// Name of the contract
        #[allow(dead_code)]
        name: String,
        /// Address of the contract on Ethereum
        #[allow(dead_code)]
        address: EthAddress,
    },
    /// Event indicating that a smart contract has been updated
    UpgradedContract {
        /// Name of the contract
        #[allow(dead_code)]
        name: String,
        /// Address of the contract on Ethereum
        #[allow(dead_code)]
        address: EthAddress,
    },
    /// Event indication a new Ethereum based token has been whitelisted for
    /// transfer across the bridge
    UpdateBridgeWhitelist {
        /// Monotonically increasing nonce
        #[allow(dead_code)]
        nonce: Uint,
        /// Tokens to be allowed to be transferred across the bridge
        #[allow(dead_code)]
        whitelist: Vec<TokenWhitelist>,
    },
}

impl EthereumEvent {
    /// SHA256 of the Borsh serialization of the [`EthereumEvent`].
    pub fn hash(&self) -> Result<Hash, std::io::Error> {
        let bytes = self.try_to_vec()?;
        Ok(Hash::sha256(bytes))
    }
}

/// An event transferring some kind of value from Ethereum to Namada
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Hash,
    Ord,
    BorshSerialize,
    BorshDeserialize,
    BorshSchema,
)]
pub struct TransferToNamada {
    /// Quantity of the ERC20 token in the transfer
    pub amount: Amount,
    /// Address of the smart contract issuing the token
    pub asset: EthAddress,
    /// The address receiving wrapped assets on Namada
    pub receiver: Address,
}

/// An event transferring some kind of value from Namada to Ethereum
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    BorshSerialize,
    BorshDeserialize,
    BorshSchema,
)]
pub struct TransferToEthereum {
    /// Quantity of wrapped Asset in the transfer
    pub amount: Amount,
    /// Address of the smart contract issuing the token
    pub asset: EthAddress,
    /// The address receiving assets on Ethereum
    pub receiver: EthAddress,
    /// The amount of fees (in NAM)
    pub gas_amount: Amount,
    /// The account of fee payer.
    pub gas_payer: Address,
}

/// struct for whitelisting a token from Ethereum.
/// Includes the address of issuing contract and
/// a cap on the max amount of this token allowed to be
/// held by the bridge.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    BorshSerialize,
    BorshDeserialize,
    BorshSchema,
)]
#[allow(dead_code)]
pub struct TokenWhitelist {
    /// Address of Ethereum smart contract issuing token
    pub token: EthAddress,
    /// Maximum amount of token allowed on the bridge
    pub cap: Amount,
}

#[cfg(test)]
pub mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_eth_address_to_canonical() {
        let canonical = testing::DAI_ERC20_ETH_ADDRESS.to_canonical();

        assert_eq!(
            testing::DAI_ERC20_ETH_ADDRESS_CHECKSUMMED.to_ascii_lowercase(),
            canonical,
        );
    }

    #[test]
    fn test_eth_address_from_str() {
        let addr =
            EthAddress::from_str(testing::DAI_ERC20_ETH_ADDRESS_CHECKSUMMED)
                .unwrap();

        assert_eq!(testing::DAI_ERC20_ETH_ADDRESS, addr);
    }

    #[test]
    fn test_eth_address_from_str_error() {
        let result = EthAddress::from_str(
            "arbitrary string which isn't an Ethereum address",
        );

        assert!(result.is_err());
    }

    /// Test that serde correct serializes EthAddress types to/from lowercase
    /// hex encodings
    #[test]
    fn test_eth_address_serde_roundtrip() {
        let addr =
            EthAddress::from_str(testing::DAI_ERC20_ETH_ADDRESS_CHECKSUMMED)
                .unwrap();
        let serialized = serde_json::to_string(&addr).expect("Test failed");
        assert_eq!(
            serialized,
            format!(
                r#""{}""#,
                testing::DAI_ERC20_ETH_ADDRESS_CHECKSUMMED.to_lowercase()
            )
        );
        let deserialized: EthAddress =
            serde_json::from_str(&serialized).expect("Test failed");
        assert_eq!(addr, deserialized);
    }
}

#[allow(missing_docs)]
/// Test helpers
#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use super::*;
    use crate::types::token::{self, Amount};

    pub const DAI_ERC20_ETH_ADDRESS_CHECKSUMMED: &str =
        "0x6B175474E89094C44Da98b954EedeAC495271d0F";
    pub const DAI_ERC20_ETH_ADDRESS: EthAddress = EthAddress([
        107, 23, 84, 116, 232, 144, 148, 196, 77, 169, 139, 149, 78, 237, 234,
        196, 149, 39, 29, 15,
    ]);
    pub const USDC_ERC20_ETH_ADDRESS_CHECKSUMMED: &str =
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
    pub const USDC_ERC20_ETH_ADDRESS: EthAddress = EthAddress([
        160, 184, 105, 145, 198, 33, 139, 54, 193, 209, 157, 74, 46, 158, 176,
        206, 54, 6, 235, 72,
    ]);

    pub fn arbitrary_eth_address() -> EthAddress {
        DAI_ERC20_ETH_ADDRESS
    }

    pub fn arbitrary_nonce() -> Uint {
        123.into()
    }

    pub fn arbitrary_keccak_hash() -> KeccakHash {
        KeccakHash([0; 32])
    }

    pub fn arbitrary_amount() -> Amount {
        Amount::from(1_000)
    }

    pub fn arbitrary_bonded_stake() -> token::Amount {
        token::Amount::from(1_000)
    }

    /// A [`EthereumEvent::TransfersToNamada`] containing a single transfer of
    /// some arbitrary ERC20
    pub fn arbitrary_single_transfer(
        nonce: Uint,
        receiver: Address,
    ) -> EthereumEvent {
        EthereumEvent::TransfersToNamada {
            nonce,
            transfers: vec![TransferToNamada {
                amount: arbitrary_amount(),
                asset: arbitrary_eth_address(),
                receiver,
            }],
        }
    }
}
