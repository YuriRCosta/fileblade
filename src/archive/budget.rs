use crate::{AppError, AppResult};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub(super) const EXPANDED_BYTES: u64 = 1024 * 1024 * 1024;
const METADATA_BYTES: u64 = 64 * 1024;

fn number(bytes: &[u8]) -> AppResult<u64> {
    if bytes.first().is_some_and(|byte| byte & 0x80 != 0) {
        let mut value = 0u64;
        for (index, byte) in bytes.iter().enumerate() {
            value = value
                .checked_mul(256)
                .and_then(|value| {
                    value.checked_add(u64::from(if index == 0 { byte & 0x7f } else { *byte }))
                })
                .ok_or_else(|| AppError::invalid("archive size exceeds its limit"))?;
        }
        return Ok(value);
    }
    let value = std::str::from_utf8(bytes)
        .map_err(|_| AppError::invalid("invalid archive size"))?
        .trim_matches(['\0', ' ']);
    u64::from_str_radix(if value.is_empty() { "0" } else { value }, 8)
        .map_err(|_| AppError::invalid("invalid archive size"))
}

pub(super) fn check(file: &mut File) -> AppResult<()> {
    file.seek(SeekFrom::Start(0))?;
    let length = file.metadata()?.len();
    if length > EXPANDED_BYTES {
        return Err(AppError::invalid("archive expansion exceeds 1 GiB"));
    }
    let mut offset = 0u64;
    let mut members = 0usize;
    let mut expanded = 0u64;
    let mut pax_size = None;
    let mut sparse_size = None;
    loop {
        let mut header = [0u8; 512];
        file.read_exact(&mut header)?;
        offset += 512;
        if header == [0u8; 512] {
            return Ok(());
        }
        members += 1;
        if members > super::ARCHIVE_MEMBER_CAP {
            return Err(AppError::invalid("archive exceeds 50000 members"));
        }
        let mut size = number(&header[124..136])?;
        if header[156] == b'x' {
            if size > METADATA_BYTES {
                return Err(AppError::invalid("archive metadata exceeds 64 KiB"));
            }
            let mut data = vec![0; size as usize];
            file.read_exact(&mut data)?;
            let mut rest = data.as_slice();
            while !rest.is_empty() {
                let space = rest
                    .iter()
                    .position(|byte| *byte == b' ')
                    .ok_or_else(|| AppError::invalid("invalid archive metadata"))?;
                let count = std::str::from_utf8(&rest[..space])
                    .ok()
                    .and_then(|text| text.parse::<usize>().ok())
                    .filter(|count| *count > space + 1 && *count <= rest.len())
                    .ok_or_else(|| AppError::invalid("invalid archive metadata length"))?;
                let field = &rest[space + 1..count];
                let equals = field.iter().position(|byte| *byte == b'=');
                if let Some(equals) = equals {
                    let key = &field[..equals];
                    if matches!(
                        key,
                        b"size" | b"GNU.sparse.realsize" | b"GNU.sparse.size" | b"SCHILY.realsize"
                    ) {
                        let value = std::str::from_utf8(&field[equals + 1..])
                            .ok()
                            .and_then(|text| text.trim_end_matches('\n').parse::<u64>().ok())
                            .ok_or_else(|| AppError::invalid("invalid expanded archive size"))?;
                        if key == b"size" {
                            pax_size = Some(value)
                        } else {
                            sparse_size = Some(value)
                        }
                    }
                }
                rest = &rest[count..];
            }
        } else {
            if !matches!(header[156], 0 | b'0' | b'1' | b'2' | b'5') {
                return Err(AppError::invalid(
                    "archive contains an unsupported special entry",
                ));
            }
            size = pax_size.take().unwrap_or(size);
            let logical = sparse_size.take().unwrap_or(size).max(size);
            expanded = expanded
                .checked_add(logical)
                .ok_or_else(|| AppError::invalid("archive expansion overflow"))?;
            if expanded > EXPANDED_BYTES {
                return Err(AppError::invalid("archive expansion exceeds 1 GiB"));
            }
        }
        let blocks = size
            .checked_add(511)
            .and_then(|size| (size / 512).checked_mul(512))
            .ok_or_else(|| AppError::invalid("archive expansion overflow"))?;
        offset = offset
            .checked_add(blocks)
            .filter(|offset| *offset <= length)
            .ok_or_else(|| AppError::invalid("archive data is incomplete"))?;
        file.seek(SeekFrom::Start(offset))?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn expanded_snapshot_has_a_fixed_byte_budget() {
        let mut file = tempfile::tempfile().unwrap();
        file.set_len(EXPANDED_BYTES + 1).unwrap();
        assert!(check(&mut file).unwrap_err().to_string().contains("1 GiB"));
    }

    #[test]
    fn empty_members_still_consume_the_member_budget() {
        let mut file = tempfile::tempfile().unwrap();
        let mut header = [0_u8; 512];
        header[0] = b'a';
        header[156] = b'0';
        for _ in 0..=super::super::ARCHIVE_MEMBER_CAP {
            file.write_all(&header).unwrap();
        }
        file.write_all(&[0; 1024]).unwrap();
        assert!(
            check(&mut file)
                .unwrap_err()
                .to_string()
                .contains("50000 members")
        );
    }
}
