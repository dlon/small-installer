use std::{fs::{self, File}, io::{BufReader, Read}, path::Path};

use anyhow::Context;
use pgp::{
    armor, packet::{Packet, PacketParser}, types::PublicKeyTrait, Deserializable, SignedPublicKey
};

const SIGNING_PUBKEY: &[u8] = include_bytes!("mullvad-code-signing.gpg");

pub fn verify(bin_path: impl AsRef<Path>, sig_path: impl AsRef<Path>) -> anyhow::Result<()> {
    let pubkey = SignedPublicKey::from_bytes(SIGNING_PUBKEY)?;
    
    let sig_reader = BufReader::new(File::open(sig_path).context("Open signature file")?);
    let signature = PacketParser::new(armor::Dearmor::new(sig_reader))
        .find_map(|packet| {
            if let Ok(Packet::Signature(sig)) = packet {
                Some(sig)
            } else {
                None
            }
        })
        .context("Missing signature")?;
    let issuer = signature
        .issuer()
        .into_iter()
        .next()
        .context("Find issuer key ID")?;

    // Find subkey used for signing
    let subkey = pubkey
        .public_subkeys
        .iter()
        .find(|subkey| &subkey.key_id() == issuer)
        .context("Find signing subkey")?;
    //subkey.verify(&pubkey)?;

    let bin = BufReader::with_capacity(1 * 1024 * 1024, File::open(bin_path)?);

    signature.verify(subkey, bin)?;

    Ok(())
}
