use crate::{Chmod, ChmodOp, ChmodTarget};
use std::result::Result as StdResult;

pub fn parse_chmod_spec(spec: &str) -> StdResult<Chmod, String> {
    let (target, rest) = if let Some(r) = spec.strip_prefix('D') {
        (ChmodTarget::Dir, r)
    } else if let Some(r) = spec.strip_prefix('F') {
        (ChmodTarget::File, r)
    } else {
        (ChmodTarget::All, spec)
    };

    if rest.is_empty() {
        return Err("missing mode".into());
    }

    if rest.chars().all(|c| c.is_ascii_digit()) {
        let bits = u32::from_str_radix(rest, 8).map_err(|_| "invalid octal mode")?;
        return Ok(Chmod {
            target,
            op: ChmodOp::Set,
            mask: 0o7777,
            bits,
            conditional: false,
        });
    }

    let (op_pos, op_char) = match rest.find(|c| c == '+' || c == '-' || c == '=') {
        Some(p) => (p, rest.as_bytes()[p] as char),
        None => {
            if let Some(ch) = rest.chars().find(|c| !matches!(*c, 'u' | 'g' | 'o' | 'a')) {
                return Err(format!("invalid operator '{ch}'"));
            } else {
                return Err("missing operator".into());
            }
        }
    };
    let who_part = &rest[..op_pos];
    let perm_part = &rest[op_pos + 1..];
    if perm_part.is_empty() {
        return Err("missing permissions".into());
    }

    let mut who_mask = 0u32;
    if who_part.is_empty() {
        who_mask = 0o777;
    } else {
        for ch in who_part.chars() {
            who_mask |= match ch {
                'u' => 0o700,
                'g' => 0o070,
                'o' => 0o007,
                'a' => 0o777,
                _ => return Err(format!("invalid class '{ch}'")),
            };
        }
    }

    let mut bits = 0u32;
    let mut mask = who_mask;
    let mut conditional = false;
    for ch in perm_part.chars() {
        match ch {
            'r' => bits |= 0o444 & who_mask,
            'w' => bits |= 0o222 & who_mask,
            'x' => bits |= 0o111 & who_mask,
            'X' => {
                bits |= 0o111 & who_mask;
                conditional = true;
            }
            's' => {
                if who_mask & 0o700 != 0 {
                    bits |= 0o4000;
                    mask |= 0o4000;
                }
                if who_mask & 0o070 != 0 {
                    bits |= 0o2000;
                    mask |= 0o2000;
                }
            }
            't' => {
                bits |= 0o1000;
                mask |= 0o1000;
            }
            _ => return Err(format!("invalid permission '{ch}'")),
        }
    }

    let op = match op_char {
        '+' => ChmodOp::Add,
        '-' => ChmodOp::Remove,
        '=' => ChmodOp::Set,
        other => return Err(format!("invalid operator '{other}'")),
    };

    Ok(Chmod {
        target,
        op,
        mask,
        bits,
        conditional,
    })
}

pub fn parse_chmod(s: &str) -> StdResult<Vec<Chmod>, String> {
    s.split(',').map(parse_chmod_spec).collect()
}

#[cfg(unix)]
use nix::unistd::{Group, User};

pub fn parse_chown(spec: &str) -> StdResult<(Option<u32>, Option<u32>), String> {
    let (user_part, group_part) = if let Some((u, g)) = spec.split_once(':') {
        (u, Some(g))
    } else {
        (spec, None)
    };

    let uid = if user_part.is_empty() {
        None
    } else {
        parse_user(user_part)?
    };

    let gid = if let Some(g) = group_part {
        if g.is_empty() {
            None
        } else {
            Some(parse_group(g)?)
        }
    } else {
        None
    };

    Ok((uid, gid))
}

#[cfg(unix)]
fn parse_user(s: &str) -> StdResult<Option<u32>, String> {
    if let Ok(id) = s.parse() {
        return Ok(Some(id));
    }
    match User::from_name(s) {
        Ok(Some(u)) => Ok(Some(u.uid.as_raw())),
        Ok(None) => Err(format!("unknown user '{s}'")),
        Err(_) => Err(format!("invalid user '{s}'")),
    }
}

#[cfg(not(unix))]
fn parse_user(s: &str) -> StdResult<Option<u32>, String> {
    let id = s.parse().map_err(|_| format!("invalid uid '{s}'"))?;
    Ok(Some(id))
}

#[cfg(unix)]
fn parse_group(s: &str) -> StdResult<u32, String> {
    if let Ok(id) = s.parse() {
        return Ok(id);
    }
    match Group::from_name(s) {
        Ok(Some(g)) => Ok(g.gid.as_raw()),
        Ok(None) => Err(format!("unknown group '{s}'")),
        Err(_) => Err(format!("invalid group '{s}'")),
    }
}

#[cfg(not(unix))]
fn parse_group(s: &str) -> StdResult<u32, String> {
    s.parse().map_err(|_| format!("invalid gid '{s}'"))
}
