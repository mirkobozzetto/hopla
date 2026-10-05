//! The palette's rows: accounts, secrets and actions, ranked against what
//! is typed.

use crate::vault::Account;

pub enum Item {
    Account { name: String, active: bool, secrets: usize },
    Secret { account: String, name: String, set: Option<bool> },
    Action(Action),
}

#[derive(Clone)]
pub enum Action {
    NewSecret(String),
    NewAccount,
    Status,
    Remove(String),
}

impl Action {
    pub fn label(&self) -> String {
        match self {
            Action::NewSecret(account) => format!("new secret in {account}"),
            Action::NewAccount => "new account".into(),
            Action::Status => "status of the active account".into(),
            Action::Remove(account) => format!("remove account {account}"),
        }
    }
}

impl Item {
    /// The text a query is matched against.
    fn haystack(&self) -> String {
        match self {
            Item::Account { name, .. } => format!("{name} account switch"),
            Item::Secret { account, name, .. } => format!("{name} {account}"),
            Item::Action(a) => a.label(),
        }
    }

    /// The account a row belongs to.
    pub fn account(&self) -> Option<&str> {
        match self {
            Item::Account { name, .. } | Item::Secret { account: name, .. } => Some(name),
            Item::Action(Action::NewSecret(a) | Action::Remove(a)) => Some(a),
            Item::Action(_) => None,
        }
    }
}

/// Case-blind: a prefix beats a word start, which beats a substring, which
/// beats letters in order. `None` when the letters are not all there.
pub fn score(hay: &str, query: &str) -> Option<i32> {
    let (hay, query) = (hay.to_lowercase(), query.to_lowercase());
    if hay.starts_with(&query) {
        return Some(400);
    }
    if let Some(i) = hay.find(&query) {
        let word = !hay[..i].ends_with(|c: char| c.is_alphanumeric());
        return Some(if word { 300 } else { 200 } - i as i32);
    }
    let mut rest = hay.chars();
    let mut gaps = 0;
    for c in query.chars() {
        loop {
            match rest.next() {
                Some(x) if x == c => break,
                Some(_) => gaps += 1,
                None => return None,
            }
        }
    }
    Some(100 - gaps)
}

/// Every account followed by its secrets, the active account first. Once
/// something is typed, actions join in and only matches stay, best first.
pub fn items(accounts: &[Account], active: Option<&str>, query: &str) -> Vec<Item> {
    let mut order: Vec<&Account> = accounts.iter().collect();
    order.sort_by_key(|a| Some(a.name.as_str()) != active);
    let mut all = Vec::new();
    for a in order {
        let is_active = Some(a.name.as_str()) == active;
        all.push(Item::Account { name: a.name.clone(), active: is_active, secrets: a.secrets.len() });
        for s in &a.secrets {
            all.push(Item::Secret { account: a.name.clone(), name: s.name.clone(), set: Some(s.set) });
        }
    }
    if query.is_empty() {
        return all;
    }
    all.extend([Item::Action(Action::NewAccount), Item::Action(Action::Status)]);
    for a in accounts {
        all.push(Item::Action(Action::NewSecret(a.name.clone())));
        if Some(a.name.as_str()) != active {
            all.push(Item::Action(Action::Remove(a.name.clone())));
        }
    }
    let mut ranked: Vec<(i32, Item)> = all.into_iter().filter_map(|i| score(&i.haystack(), query).map(|s| (s, i))).collect();
    ranked.sort_by_key(|(s, _)| -s);
    ranked.into_iter().map(|(_, i)| i).collect()
}
