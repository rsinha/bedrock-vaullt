//! Here we implement the Schnorr signature scheme over the prime order subgroup of the JubJub curve
//! (defined in zCash sapling). This curve is also known as ed-on-bls12-381 in arkworks.

use super::SignatureScheme;
use ark_crypto_primitives::Error;
use ark_std::ops::*;
use ark_ec::{CurveConfig, CurveGroup};
use ark_ff::{
    fields::PrimeField,
    UniformRand,
};
use ark_serialize::CanonicalSerialize;
use ark_std::rand::Rng;
use ark_std::{hash::Hash, marker::PhantomData, vec::Vec};
use blake2::Blake2s;
use digest::Digest;

pub struct Schnorr<C: CurveGroup> {
    _group: PhantomData<C>,
}

#[derive(Clone, Debug)]
pub struct Parameters<C: CurveGroup> {
    pub generator: C::Affine,
    pub salt: Option<[u8; 32]>,
}

pub type PublicKey<C> = <C as CurveGroup>::Affine;

pub type SecretKey<C> = <<C as CurveGroup>::Config as CurveConfig>::ScalarField;

#[derive(Clone, Default, Debug, CanonicalSerialize)]
pub struct Signature<C: CurveGroup> {
    pub prover_response: C::ScalarField,
    pub verifier_challenge: [u8; 32],
}

impl<C: CurveGroup + Hash> SignatureScheme for Schnorr<C>
where
    C::ScalarField: PrimeField,
{
    type Parameters = Parameters<C>;
    type PublicKey = PublicKey<C>;
    type SecretKey = SecretKey<C>;
    type Signature = Signature<C>;

    fn setup<R: Rng>(_rng: &mut R) -> Result<Self::Parameters, Error> {
        // let setup_time = start_timer!(|| "SchnorrSig::Setup");

        let salt = None;
        let generator = C::generator().into();

        // end_timer!(setup_time);
        Ok(Parameters { generator, salt })
    }

    fn keygen<R: Rng>(
        parameters: &Self::Parameters,
        rng: &mut R,
    ) -> Result<(Self::PublicKey, Self::SecretKey), Error> {
        // let keygen_time = start_timer!(|| "SchnorrSig::KeyGen");

        // Secret is a random scalar x
        // the pubkey is y = xG
        let secret_key = C::ScalarField::rand(rng);
        let public_key = parameters.generator.mul(secret_key).into();

        // end_timer!(keygen_time);
        Ok((
            public_key,
            secret_key
        ))
    }

    fn sign<R: Rng>(
        parameters: &Self::Parameters,
        sk: &Self::SecretKey,
        message: &[u8],
        rng: &mut R,
    ) -> Result<Self::Signature, Error> {
        // let sign_time = start_timer!(|| "SchnorrSig::Sign");
        // (k, e);
        let (random_scalar, verifier_challenge) = {
            // Sample a random scalar `k` from the prime scalar field.
            let random_scalar: C::ScalarField = C::ScalarField::rand(rng);
            // Commit to the random scalar via r := k · G.
            // This is the prover's first msg in the Sigma protocol.
            let prover_commitment = parameters.generator.mul(random_scalar).into_affine();

            let public_key = parameters.generator.mul(sk).into();
            // Hash everything to get verifier challenge.
            // e := H(salt || pubkey || r || msg);
            let mut hash_input = Vec::new();
            if parameters.salt != None {
               parameters.salt.serialize_compressed(&mut hash_input)?;
            }
            public_key.serialize_compressed(&mut hash_input)?;
            prover_commitment.serialize_compressed(&mut hash_input)?;
            message.serialize_compressed(&mut hash_input)?;

            let hash_digest = Blake2s::digest(&hash_input);
            assert!(hash_digest.len() >= 32);
            let mut verifier_challenge = [0u8; 32];
            verifier_challenge.copy_from_slice(&hash_digest.as_slice());

            (random_scalar, verifier_challenge)
        };

        let verifier_challenge_fe = C::ScalarField::from_le_bytes_mod_order(&verifier_challenge);

        // k - xe;
        let prover_response = random_scalar - (verifier_challenge_fe * sk);
        let signature = Signature {
            prover_response,
            verifier_challenge,
        };

        // end_timer!(sign_time);
        Ok(signature)
    }

    fn verify(
        parameters: &Self::Parameters,
        pk: &Self::PublicKey,
        message: &[u8],
        signature: &Self::Signature,
    ) -> Result<bool, Error> {
        // let verify_time = start_timer!(|| "SchnorrSig::Verify");

        let Signature {
            prover_response,
            verifier_challenge,
        } = signature;
        let verifier_challenge_fe = C::ScalarField::from_le_bytes_mod_order(verifier_challenge);
        // sG = kG - eY
        // kG = sG + eY
        // so we first solve for kG.
        let mut claimed_prover_commitment = parameters.generator.mul(*prover_response);
        let public_key_times_verifier_challenge = pk.mul(verifier_challenge_fe);
        claimed_prover_commitment += &public_key_times_verifier_challenge;
        let claimed_prover_commitment = claimed_prover_commitment.into_affine();

        // e := H(salt || pubkey || r || msg)
        let mut hash_input = Vec::new();
        if parameters.salt != None {
            hash_input.extend_from_slice(&parameters.salt.unwrap());
        }
        pk.serialize_compressed(&mut hash_input)?;
        claimed_prover_commitment.serialize_compressed(&mut hash_input)?;
        message.serialize_compressed(&mut hash_input)?;

        // cast the hash output to get e
        let obtained_verifier_challenge = &Blake2s::digest(&hash_input)[..];
        // end_timer!(verify_time);
        // The signature is valid iff the computed verifier challenge is the same as the one
        // provided in the signature
        Ok(verifier_challenge == obtained_verifier_challenge)
    }
}
