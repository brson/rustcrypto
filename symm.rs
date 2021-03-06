use libc::{c_int, c_uint};

export encryptmode, decryptmode;
export encrypt, decrypt;
export libcrypto;

#[allow(non_camel_case_types)]
type EVP_CIPHER_CTX = *libc::c_void;

#[allow(non_camel_case_types)]
type EVP_CIPHER = *libc::c_void;

#[link_name = "crypto"]
#[abi = "cdecl"]
extern mod libcrypto {
    fn EVP_CIPHER_CTX_new() -> EVP_CIPHER_CTX;
    fn EVP_CIPHER_CTX_set_padding(ctx: EVP_CIPHER_CTX, padding: c_int);

    fn EVP_aes_128_ecb() -> EVP_CIPHER;
    fn EVP_aes_128_cbc() -> EVP_CIPHER;
    fn EVP_aes_192_ecb() -> EVP_CIPHER;
    fn EVP_aes_192_cbc() -> EVP_CIPHER;
    fn EVP_aes_256_ecb() -> EVP_CIPHER;
    fn EVP_aes_256_cbc() -> EVP_CIPHER;

    fn EVP_CipherInit(ctx: EVP_CIPHER_CTX, evp: EVP_CIPHER,
                      key: *u8, iv: *u8, mode: c_int);
    fn EVP_CipherUpdate(ctx: EVP_CIPHER_CTX, outbuf: *mut u8,
                        outlen: &mut c_uint, inbuf: *u8, inlen: c_int);
    fn EVP_CipherFinal(ctx: EVP_CIPHER_CTX, res: *mut u8, len: &mut c_int);
}

pub enum Mode {
    Encrypt,
    Decrypt,
}

#[allow(non_camel_case_types)]
pub enum Type {
    AES_256_ECB,
    AES_256_CBC,
}

fn evpc(t: Type) -> (EVP_CIPHER, uint, uint) {
    match t {
        AES_256_ECB => (libcrypto::EVP_aes_256_ecb(), 32u, 16u),
        AES_256_CBC => (libcrypto::EVP_aes_256_cbc(), 32u, 16u),
    }
}

/// Represents a symmetric cipher context.
pub struct Crypter {
    priv evp: EVP_CIPHER,
    priv ctx: EVP_CIPHER_CTX,
    priv keylen: uint,
    priv blocksize: uint
}

pub fn Crypter(t: Type) -> Crypter {
    let ctx = libcrypto::EVP_CIPHER_CTX_new();
    let (evp, keylen, blocksz) = evpc(t);
    Crypter { evp: evp, ctx: ctx, keylen: keylen, blocksize: blocksz }
}

pub impl Crypter {
    /**
     * Enables or disables padding. If padding is disabled, total amount of
     * data encrypted must be a multiple of block size.
     */
    fn pad(padding: bool) {
        let v = if padding { 1 } else { 0} as c_int;
        libcrypto::EVP_CIPHER_CTX_set_padding(self.ctx, v);
    }

    /**
     * Initializes this crypter.
     */
    fn init(mode: Mode, key: &[u8], iv: &[u8]) unsafe {
        let mode = match mode {
            Encrypt => 1 as c_int,
            Decrypt => 0 as c_int,
        };
        assert key.len() == self.keylen;

        do vec::as_imm_buf(key) |pkey, _len| {
            do vec::as_imm_buf(iv) |piv, _len| {
                libcrypto::EVP_CipherInit(
                    self.ctx,
                    self.evp,
                    pkey,
                    piv,
                    mode
                )
            }
        }
    }

    /**
     * Update this crypter with more data to encrypt or decrypt. Returns
     * encrypted or decrypted bytes.
     */
    fn update(data: &[u8]) -> ~[u8] unsafe {
        do vec::as_imm_buf(data) |pdata, len| {
            let mut res = vec::from_elem(len + self.blocksize, 0u8);

            let reslen = do vec::as_mut_buf(res) |pres, _len| {
                let mut reslen = (len + self.blocksize) as u32;

                libcrypto::EVP_CipherUpdate(
                    self.ctx,
                    pres,
                    &mut reslen,
                    pdata,
                    len as c_int
                );

                reslen
            };

            vec::slice(res, 0u, reslen as uint)
        }
    }

    /**
     * Finish crypting. Returns the remaining partial block of output, if any.
     */
    fn final() -> ~[u8] unsafe {
        let res = vec::to_mut(vec::from_elem(self.blocksize, 0u8));

        let reslen = do vec::as_mut_buf(res) |pres, _len| {
            let mut reslen = self.blocksize as c_int;
            libcrypto::EVP_CipherFinal(self.ctx, pres, &mut reslen);
            reslen
        };

        vec::slice(res, 0u, reslen as uint)
    }
}

/**
 * Encrypts data, using the specified crypter type in encrypt mode with the
 * specified key and iv; returns the resulting (encrypted) data.
 */
fn encrypt(t: Type, key: &[u8], iv: ~[u8], data: &[u8]) -> ~[u8] {
    let c = Crypter(t);
    c.init(Encrypt, key, iv);
    let r = c.update(data);
    let rest = c.final();
    r + rest
}

/**
 * Decrypts data, using the specified crypter type in decrypt mode with the
 * specified key and iv; returns the resulting (decrypted) data.
 */
fn decrypt(t: Type, key: &[u8], iv: ~[u8], data: &[u8]) -> ~[u8] {
    let c = Crypter(t);
    c.init(Decrypt, key, iv);
    let r = c.update(data);
    let rest = c.final();
    r + rest
}

#[cfg(test)]
mod tests {
    // Test vectors from FIPS-197:
    // http://csrc.nist.gov/publications/fips/fips197/fips-197.pdf
    #[test]
    fn test_aes_256_ecb() {
        let k0 =
           ~[ 0x00u8, 0x01u8, 0x02u8, 0x03u8, 0x04u8, 0x05u8, 0x06u8, 0x07u8,
              0x08u8, 0x09u8, 0x0au8, 0x0bu8, 0x0cu8, 0x0du8, 0x0eu8, 0x0fu8,
              0x10u8, 0x11u8, 0x12u8, 0x13u8, 0x14u8, 0x15u8, 0x16u8, 0x17u8,
              0x18u8, 0x19u8, 0x1au8, 0x1bu8, 0x1cu8, 0x1du8, 0x1eu8, 0x1fu8 ];
        let p0 =
           ~[ 0x00u8, 0x11u8, 0x22u8, 0x33u8, 0x44u8, 0x55u8, 0x66u8, 0x77u8,
              0x88u8, 0x99u8, 0xaau8, 0xbbu8, 0xccu8, 0xddu8, 0xeeu8, 0xffu8 ];
        let c0 =
           ~[ 0x8eu8, 0xa2u8, 0xb7u8, 0xcau8, 0x51u8, 0x67u8, 0x45u8, 0xbfu8,
              0xeau8, 0xfcu8, 0x49u8, 0x90u8, 0x4bu8, 0x49u8, 0x60u8, 0x89u8 ];
        let c = Crypter(AES_256_ECB);
        c.init(Encrypt, k0, ~[]);
        c.pad(false);
        let r0 = c.update(p0) + c.final();
        assert(r0 == c0);
        c.init(Decrypt, k0, ~[]);
        c.pad(false);
        let p1 = c.update(r0) + c.final();
        assert(p1 == p0);
    }
}
