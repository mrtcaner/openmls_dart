//! Allocation-free profile-specific TLS shape bounds. OpenMLS still performs all
//! cryptographic and semantic validation; this scanner never authenticates data.
use super::{Error, MAX_AAD_BYTES, MAX_LEAVES, MAX_MLS_BYTES, MAX_TREE_NODES};

struct Cursor<'a> {
    bytes: &'a [u8],
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let (a, b) = self
            .bytes
            .split_at_checked(n)
            .ok_or(Error::DecodeRejected)?;
        self.bytes = b;
        Ok(a)
    }
    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, Error> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    fn vector(&mut self, max: usize) -> Result<&'a [u8], Error> {
        let first = self.byte()?;
        let tag = first >> 6;
        if tag == 3 {
            return Err(Error::DecodeRejected);
        }
        let width = 1usize << tag;
        let mut n = (first & 63) as usize;
        for b in self.take(width - 1)? {
            n = (n << 8) | (*b as usize);
        }
        if (width == 2 && n < 64) || (width == 4 && n < 16384) {
            return Err(Error::NoncanonicalEncoding);
        }
        if n > max {
            return Err(Error::LimitExceeded);
        }
        self.take(n)
    }
    fn exact(&mut self, n: usize) -> Result<(), Error> {
        if self.vector(n)?.len() != n {
            return Err(Error::DecodeRejected);
        }
        Ok(())
    }
    fn finish(self) -> Result<(), Error> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(Error::DecodeRejected)
        }
    }
}
fn extensions(c: &mut Cursor<'_>) -> Result<(), Error> {
    let mut e = Cursor {
        bytes: c.vector(16 * 1024)?,
    };
    let mut count = 0;
    while !e.bytes.is_empty() {
        count += 1;
        if count > 32 {
            return Err(Error::LimitExceeded);
        }
        e.u16()?;
        e.vector(4096)?;
    }
    Ok(())
}
fn leaf(c: &mut Cursor<'_>) -> Result<(), Error> {
    c.exact(32)?;
    c.exact(32)?;
    if c.u16()? != 1 {
        return Err(Error::UnsupportedProfile);
    }
    c.exact(45)?;
    for _ in 0..5 {
        if c.vector(128)?.len() % 2 != 0 {
            return Err(Error::DecodeRejected);
        }
    }
    match c.byte()? {
        1 => {
            c.take(16)?;
        }
        2 => {}
        3 => {
            c.vector(32)?;
        }
        _ => return Err(Error::DecodeRejected),
    }
    extensions(c)?;
    c.exact(64)
}
fn tree(bytes: &[u8]) -> Result<(), Error> {
    let mut outer = Cursor { bytes };
    let mut nodes = Cursor {
        bytes: outer.vector(MAX_MLS_BYTES)?,
    };
    outer.finish()?;
    let mut count = 0;
    let mut leaves = 0;
    while !nodes.bytes.is_empty() {
        count += 1;
        if count > MAX_TREE_NODES {
            return Err(Error::LimitExceeded);
        }
        match nodes.byte()? {
            0 => {}
            1 => match nodes.byte()? {
                1 => {
                    leaves += 1;
                    if leaves > MAX_LEAVES {
                        return Err(Error::LimitExceeded);
                    }
                    leaf(&mut nodes)?;
                }
                2 => {
                    nodes.exact(32)?;
                    nodes.vector(32)?;
                    let unmerged = nodes.vector(MAX_LEAVES * 4)?;
                    if unmerged.len() % 4 != 0 {
                        return Err(Error::DecodeRejected);
                    }
                    for b in unmerged.as_chunks::<4>().0 {
                        if u32::from_be_bytes([b[0], b[1], b[2], b[3]]) >= MAX_LEAVES as u32 {
                            return Err(Error::LimitExceeded);
                        }
                    }
                }
                _ => return Err(Error::DecodeRejected),
            },
            _ => return Err(Error::DecodeRejected),
        }
    }
    if count == 0 || leaves == 0 {
        return Err(Error::TreeRejected);
    }
    Ok(())
}
pub(super) fn group_info(bytes: &[u8]) -> Result<(), Error> {
    if bytes.is_empty() || bytes.len() > MAX_MLS_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut c = Cursor { bytes };
    if c.u16()? != 1 || c.u16()? != 4 || c.u16()? != 1 || c.u16()? != 1 {
        return Err(Error::UnsupportedProfile);
    }
    c.exact(16)?;
    let epoch = c.take(8)?;
    c.exact(32)?;
    let transcript = c.vector(32)?;
    if transcript.len() != 32 && !(epoch == [0; 8] && transcript.is_empty()) {
        return Err(Error::DecodeRejected);
    }
    if !c.vector(16 * 1024)?.is_empty() {
        return Err(Error::UnsupportedProfile);
    }
    let mut e = Cursor {
        bytes: c.vector(MAX_MLS_BYTES)?,
    };
    let mut seen_tree = false;
    let mut seen_pub = false;
    while !e.bytes.is_empty() {
        let kind = e.u16()?;
        let body = e.vector(MAX_MLS_BYTES)?;
        match kind {
            2 if !seen_tree => {
                tree(body)?;
                seen_tree = true;
            }
            4 if !seen_pub => {
                let mut p = Cursor { bytes: body };
                p.exact(32)?;
                p.finish()?;
                seen_pub = true;
            }
            _ => return Err(Error::UnsupportedProfile),
        }
    }
    if !seen_tree || !seen_pub {
        return Err(Error::MissingRequiredExtension);
    }
    c.exact(32)?;
    c.take(4)?;
    c.exact(64)?;
    c.finish()
}
pub(crate) fn external_commit(bytes: &[u8]) -> Result<(), Error> {
    if bytes.is_empty() || bytes.len() > MAX_MLS_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut c = Cursor { bytes };
    if c.u16()? != 1 || c.u16()? != 1 {
        return Err(Error::UnsupportedProfile);
    }
    c.exact(16)?;
    c.take(8)?;
    if c.byte()? != 4 {
        return Err(Error::SignerMismatch);
    }
    if c.vector(MAX_AAD_BYTES)?.is_empty() {
        return Err(Error::LimitExceeded);
    }
    if c.byte()? != 3 {
        return Err(Error::ProtocolRejected);
    }
    let mut props = Cursor {
        bytes: c.vector(128)?,
    };
    let mut init = 0;
    let mut remove = 0;
    while !props.bytes.is_empty() {
        if props.byte()? != 1 {
            return Err(Error::DeltaMismatch);
        }
        match props.u16()? {
            6 if init == 0 => {
                props.exact(32)?;
                init += 1;
            }
            3 if remove == 0 => {
                props.take(4)?;
                remove += 1;
            }
            _ => return Err(Error::DeltaMismatch),
        }
    }
    if init != 1 || remove != 1 {
        return Err(Error::DeltaMismatch);
    }
    if c.byte()? != 1 {
        return Err(Error::ProtocolRejected);
    }
    leaf(&mut c)?;
    update_path(c.vector(MAX_MLS_BYTES)?)?;
    c.exact(64)?;
    c.exact(32)?;
    c.finish()
}
fn update_path(bytes: &[u8]) -> Result<(), Error> {
    let mut path = Cursor { bytes };
    let mut depth = 0;
    while !path.bytes.is_empty() {
        depth += 1;
        if depth > 8 {
            return Err(Error::LimitExceeded);
        }
        path.exact(32)?;
        let mut recipients = Cursor {
            bytes: path.vector(MAX_MLS_BYTES)?,
        };
        let mut count = 0;
        while !recipients.bytes.is_empty() {
            count += 1;
            if count > MAX_LEAVES {
                return Err(Error::LimitExceeded);
            }
            recipients.exact(32)?;
            recipients.exact(48)?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn vector(b: &[u8]) -> Vec<u8> {
        let n = b.len() as u32;
        let mut v = if n < 64 {
            vec![n as u8]
        } else if n < 16384 {
            (n as u16 | 0x4000).to_be_bytes().to_vec()
        } else {
            (n | 0x80000000).to_be_bytes().to_vec()
        };
        v.extend_from_slice(b);
        v
    }
    #[test]
    fn tls_vectors_and_lists_are_bounded_before_decode() {
        assert_eq!(
            Cursor {
                bytes: &[0x40, 1, 0]
            }
            .vector(128),
            Err(Error::NoncanonicalEncoding)
        );
        assert_eq!(
            Cursor { bytes: &[0xc0] }.vector(128),
            Err(Error::DecodeRejected)
        );
        assert_eq!(
            Cursor {
                bytes: &[0x80, 0x00, 0x40, 0x00]
            }
            .vector(128),
            Err(Error::LimitExceeded)
        );
        assert_eq!(
            Cursor { bytes: &[0x40] }.vector(128),
            Err(Error::DecodeRejected)
        );
        assert_eq!(
            tree(&vector(&vec![0; MAX_TREE_NODES + 1])),
            Err(Error::LimitExceeded)
        );
        assert_eq!(
            tree(&vector(&vec![0; MAX_TREE_NODES])),
            Err(Error::TreeRejected)
        );
        let parent = |unmerged: &[u8]| {
            let mut p = vec![1, 2];
            p.extend(vector(&[0; 32]));
            p.push(0);
            p.extend(vector(unmerged));
            vector(&p)
        };
        assert_eq!(
            tree(&parent(&vec![0; (MAX_LEAVES + 1) * 4])),
            Err(Error::LimitExceeded)
        );
        assert_eq!(
            tree(&parent(&(MAX_LEAVES as u32).to_be_bytes())),
            Err(Error::LimitExceeded)
        );
        assert_eq!(tree(&parent(&[1])), Err(Error::DecodeRejected));
        let list = vector(&[0, 1, 0].repeat(33));
        assert_eq!(
            extensions(&mut Cursor { bytes: &list }),
            Err(Error::LimitExceeded)
        );
    }
    #[test]
    fn update_path_depth_and_recipient_counts_are_bounded() {
        let node = |recipients: &[u8]| {
            let mut n = vector(&[0; 32]);
            n.extend(vector(recipients));
            n
        };
        assert_eq!(update_path(&node(&[]).repeat(8)), Ok(()));
        assert_eq!(update_path(&node(&[]).repeat(9)), Err(Error::LimitExceeded));
        let mut ciphertext = vector(&[0; 32]);
        ciphertext.extend(vector(&[0; 48]));
        assert_eq!(update_path(&node(&ciphertext.repeat(MAX_LEAVES))), Ok(()));
        assert_eq!(
            update_path(&node(&ciphertext.repeat(MAX_LEAVES + 1))),
            Err(Error::LimitExceeded)
        );
        let mut truncated = node(&ciphertext);
        truncated.pop();
        assert_eq!(update_path(&truncated), Err(Error::DecodeRejected));
    }
}
