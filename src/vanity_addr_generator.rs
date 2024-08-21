//! # Vanity Address Generation Module
//!
//! This module is used to generate Bitcoin vanity addresses with multithreading.
//!
//! # Example Usage
//!
//! ```rust
//! use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
//!
//! let vanity_address = VanityAddr::generate(
//!             "Test", // the string that you want your vanity address include.
//!             16, // number of threads
//!             false, // case sensitivity (false ex: tESt, true ex: Test)
//!             true, // fast mode flag (to use a string longer than 4 chars this must be set to false)
//!             VanityMode::Anywhere, // vanity mode flag (prefix, suffix, anywhere available)
//!             ).unwrap(); // this function returns a result type
//!
//! println!("private_key (wif): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n\n",
//!                 vanity_address.get_wif_private_key(),
//!                 vanity_address.get_comp_public_key(),
//!                 vanity_address.get_comp_address())
//! ```

use crate::error::VanitiyGeneretorError;
use crate::keys_and_address::KeysAndAddress;
use bitcoin::secp256k1::{All, Secp256k1};
use num_bigint::BigUint;
use num_traits::Num;
use std::sync::mpsc;
use std::thread;

/// An Empty Struct for a more structured code
/// implements the only public function generate
pub struct VanityAddr;

/// Vanity mode enum
#[derive(Copy, Clone, Debug)]
pub enum VanityMode {
    Prefix,
    Suffix,
    Anywhere,
}

impl VanityAddr {
    /// Checks all given information's before passing to the vanity address finder function.
    /// Returns Ok if all checks were successful.
    /// Returns Err if the string if longer than 4 chars and -d or --disable-fast-mode flags are not given.
    /// Returns Err if the string is not in base58 format.
    fn validate_input(string: &str, fast_mode: bool) -> Result<(), VanitiyGeneretorError> {
        if string.is_empty() {
            return Ok(());
        }

        if string.len() > 4 && fast_mode {
            return Err(VanitiyGeneretorError(
                    "You're asking for too much!\n\
                    If you know this will take for a long time and really want to find something longer than 4 characters\n\
                    disable fast mode with -df or --disable_fast flags.",
                ));
        }

        let is_base58 = string
            .chars()
            .any(|c| c == '0' || c == 'I' || c == 'O' || c == 'l' || !c.is_alphanumeric());

        if is_base58 {
            return Err(VanitiyGeneretorError(
                    "Your input is not in base58. Don't include zero: '0', uppercase i: 'I', uppercase o: 'O', lowercase L: 'l'\
                    or any non-alphanumeric character in your input!",
                ));
        }

        Ok(())
    }

    /// Checks all given information's before passing to the vanity address finder function.
    /// Returns Result<KeysAndAddressString, VanitiyGeneretorError>
    /// Returns OK if a vanity address found successfully with keys_and_address::KeysAndAddress struct
    /// Returns Err if the string if longer than 4 chars and -d or --disable-fast-mode flags are not given.
    /// Returns Err if the string is not in base58 format.
    pub fn generate(
        string: &str,
        threads: u64,
        case_sensitive: bool,
        fast_mode: bool,
        vanity_mode: VanityMode,
    ) -> Result<KeysAndAddress, VanitiyGeneretorError> {
        let secp256k1 = Secp256k1::new();

        Self::validate_input(string, fast_mode)?;

        if string.is_empty() {
            return Ok(KeysAndAddress::generate_random(&secp256k1));
        }

        Ok(SearchEngines::find_vanity_address(
            string,
            threads,
            case_sensitive,
            vanity_mode,
            secp256k1,
        ))
    }

    /// Checks all given information's before passing to the vanity address finder function.
    /// Returns Result<KeysAndAddressString, VanitiyGeneretorError>
    /// Returns OK if a vanity address found successfully with keys_and_address::KeysAndAddress struct
    /// Returns Err if the string if longer than 4 chars and -d or --disable-fast-mode flags are not given.
    /// Returns Err if the string is not in base58 format.
    /// Returns Err if something wen't wrong while generating keypair within range
    pub fn generate_within_range(
        string: &str,
        range_min: BigUint,
        range_max: BigUint,
        threads: u64,
        case_sensitive: bool,
        fast_mode: bool,
        vanity_mode: VanityMode,
    ) -> Result<KeysAndAddress, VanitiyGeneretorError> {
        let secp256k1 = Secp256k1::new();

        Self::validate_input(string, fast_mode)?;

        if string.is_empty() {
            return Ok(KeysAndAddress::generate_within_range(
                &secp256k1, &range_min, &range_max, true,
            )?);
        }

        SearchEngines::find_vanity_address_within_range(
            string,
            range_min,
            range_max,
            threads,
            case_sensitive,
            vanity_mode,
            secp256k1,
        )
    }
}

/// impl's `find_vanity_address_fast_engine` and `find_vanity_address_fast_engine_with_range`
pub struct SearchEngines;

impl SearchEngines {
    /// Search for the vanity address with given threads.
    /// First come served! If a thread finds a vanity address that satisfy all the requirements it sends
    /// the keys_and_address::KeysAndAddress struct wia std::sync::mpsc channel and find_vanity_address function kills all of the other
    /// threads and closes the channel and returns the found KeysAndAddress struct that includes
    /// key pair and the desired address.
    fn find_vanity_address(
        string: &str,
        threads: u64,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        secp256k1: Secp256k1<All>,
    ) -> KeysAndAddress {
        let string_len = string.len();
        let (sender, receiver) = mpsc::channel();

        for _ in 0..threads {
            let sender = sender.clone();
            let string = string.to_string();
            let mut anywhere_flag = false;
            let mut prefix_suffix_flag = false;
            let secp256k1 = secp256k1.clone();

            let _ = thread::spawn(move || {
                loop {
                    let keys_and_address = KeysAndAddress::generate_random(&secp256k1);
                    let address = keys_and_address.get_comp_address();

                    match vanity_mode {
                        VanityMode::Prefix => {
                            let slice = &address[1..=string_len];
                            prefix_suffix_flag = match case_sensitive {
                                true => slice == string,
                                false => slice.to_lowercase() == string.to_lowercase(),
                            };
                        }
                        VanityMode::Suffix => {
                            let address_len = address.len();
                            let slice = &address[address_len - string_len..address_len];
                            prefix_suffix_flag = match case_sensitive {
                                true => slice == string,
                                false => slice.to_lowercase() == string.to_lowercase(),
                            };
                        }
                        VanityMode::Anywhere => {
                            anywhere_flag = match case_sensitive {
                                true => address.contains(&string),
                                false => address.to_lowercase().contains(&string.to_lowercase()),
                            };
                        }
                    }
                    // If the channel closed, that means another thread found a keypair and closed it
                    // so we just return and kill the thread if an error occurs.
                    if (prefix_suffix_flag || anywhere_flag)
                        && sender.send(keys_and_address).is_err()
                    {
                        return;
                    }
                }
            });
        }

        loop {
            match receiver.try_recv() {
                Ok(pair) => return pair,
                Err(_) => continue,
            }
        }
    }

    /// Search for the vanity address with given threads, which private key is within given range.
    /// First come served! If a thread finds a vanity address that satisfy all the requirements it sends
    /// the keys_and_address::KeysAndAddress struct wia std::sync::mpsc channel and find_vanity_address function kills all of the other
    /// threads and closes the channel and returns the found KeysAndAddress struct that includes
    /// key pair and the desired address.
    fn find_vanity_address_within_range(
        string: &str,
        range_min: BigUint,
        range_max: BigUint,
        threads: u64,
        case_sensitive: bool,
        vanity_mode: VanityMode,
        secp256k1: Secp256k1<All>,
    ) -> Result<KeysAndAddress, VanitiyGeneretorError> {
        let string_len = string.len();
        let (sender, receiver) = mpsc::channel();

        // Ensure range_max is greater than range_min
        if range_max < range_min {
            return Err(VanitiyGeneretorError(
                "range_max must be greater than range_min",
            ));
        }

        // Private key range_max must be within the valid range for Secp256k1
        let secp256k1_order = BigUint::from_str_radix(
            "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
            16,
        )
        .map_err(|_| VanitiyGeneretorError("Failed to parse hexadecimal string"))?;

        if range_max > secp256k1_order {
            return Err(VanitiyGeneretorError(
                "range_max must be within the valid range for Secp256k1",
            ));
        }

        for _ in 0..threads {
            let sender = sender.clone();
            let string = string.to_string();
            let mut anywhere_flag = false;
            let mut prefix_suffix_flag = false;
            let secp256k1 = secp256k1.clone();
            let range_min = range_min.clone();
            let range_max = range_max.clone();

            let _ = thread::spawn(move || {
                loop {
                    let keys_and_address = KeysAndAddress::generate_within_range(
                        &secp256k1, &range_min, &range_max, false,
                    )
                    .unwrap();
                    let address = keys_and_address.get_comp_address();

                    match vanity_mode {
                        VanityMode::Prefix => {
                            let slice = &address[1..=string_len];
                            prefix_suffix_flag = match case_sensitive {
                                true => slice == string,
                                false => slice.to_lowercase() == string.to_lowercase(),
                            };
                        }
                        VanityMode::Suffix => {
                            let address_len = address.len();
                            let slice = &address[address_len - string_len..address_len];
                            prefix_suffix_flag = match case_sensitive {
                                true => slice == string,
                                false => slice.to_lowercase() == string.to_lowercase(),
                            };
                        }
                        VanityMode::Anywhere => {
                            anywhere_flag = match case_sensitive {
                                true => address.contains(&string),
                                false => address.to_lowercase().contains(&string.to_lowercase()),
                            };
                        }
                    }
                    // If the channel closed, that means another thread found a keypair and closed it
                    // so we just return and kill the thread if an error occurs.
                    if (prefix_suffix_flag || anywhere_flag)
                        && sender.send(keys_and_address).is_err()
                    {
                        return;
                    }
                }
            });
        }

        loop {
            match receiver.try_recv() {
                Ok(pair) => return Ok(pair),
                Err(_) => continue,
            }
        }
    }
}
