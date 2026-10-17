<p align="center">
  <img src="https://raw.githubusercontent.com/mirkobozzetto/hopla/main/assets/logo.svg" alt="hopla" width="640">
</p>

<p align="center">
  <b>Your GitHub accounts and their secrets, one keystroke away.</b><br>
  A terminal palette for <a href="https://github.com/mirkobozzetto/hop">hop</a>,
  guarded by Touch ID and a bunny in a top hat.
</p>

<p align="center">
  <a href="https://github.com/mirkobozzetto/hopla/releases/latest"><img src="https://img.shields.io/github/v/release/mirkobozzetto/hopla?color=f8b800&label=release" alt="release"></a>
  <a href="https://crates.io/crates/hopla"><img src="https://img.shields.io/crates/v/hopla?color=f472b6" alt="crates.io"></a>
  <img src="https://img.shields.io/badge/macOS%20·%20Linux%20·%20Windows-a78bfa" alt="macOS, Linux, Windows">
  <img src="https://img.shields.io/badge/license-MIT-3cbcfc" alt="MIT">
</p>

<p align="center">
  <img src="https://raw.githubusercontent.com/mirkobozzetto/hopla/main/assets/palette.png" alt="hopla's palette" width="760">
</p>

## Why

You have a work GitHub account and a personal one, each with its own SSH
key, its own `gh` login and its own API keys. [hop](https://github.com/mirkobozzetto/hop)
switches all of it in one word. hopla puts all of it in one list you type
into: accounts, secrets, actions. Find, Enter, done.

A value never shows on screen. It goes straight to the clipboard, hidden
from clipboard history apps, and leaves it 30 seconds later.

## Install

macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/mirkobozzetto/hopla/main/install.sh | sh
```

Windows, in PowerShell:

```powershell
irm https://raw.githubusercontent.com/mirkobozzetto/hopla/main/install.ps1 | iex
```

With Cargo:

```sh
cargo install hopla
```

hopla reads the accounts that [hop](https://github.com/mirkobozzetto/hop)
sets up: install hop first. Then run `hopla`, or `hop ui`. On Windows, hop
and hopla need Git Bash.

hopla works on the account of the terminal it starts in: the one a `.hop`
file names for that folder, or the one `hop <account>` picked there. A
switch in hopla changes that terminal only, the others keep theirs. See
[one terminal at a time](https://github.com/mirkobozzetto/hop#one-terminal-at-a-time).

## Use

<p align="center">
  <img src="https://raw.githubusercontent.com/mirkobozzetto/hopla/main/assets/lock.png" alt="hopla's lock screen" width="640">
</p>

Unlock with Touch ID, Windows Hello or your password, then type. Accounts,
secrets and actions (`new account`, `status`, `remove account wombat`)
filter as you go: a prefix wins, then a word start, then letters in order.

| Key | What it does |
| --- | --- |
| type | filter accounts, secrets and actions |
| `↵` | the obvious thing: switch account, copy a secret, fill an empty one, run an action |
| `↑` `↓` `tab` | move |
| `^n` | new secret in the row's account |
| `^e` | replace a value |
| `^d` | delete a secret or remove an account, after Touch ID again |
| `^b` | switch to the row's account, like `hop <account>` |
| `^s` | `hop s`: who GitHub thinks you are |
| `^a` | new account |
| `^l` | lock |
| `esc` | clear the search, then quit |

Values are typed blind, one `*` per character, with the rules of
`hop secret set`. hopla locks itself after 5 minutes without a key.

## A new account in one form

`^a` asks for a name, an email, an SSH key and a GitHub login. Then hopla:

1. adds the account to hop's `.env`,
2. makes the SSH key if it does not exist,
3. runs hop's `install.sh`,
4. copies the public key and opens GitHub's page to paste it,
5. runs `gh auth login` for that account.

Removing an account keeps its secrets: create it again and they are back.

## Where your secrets live

| | macOS | Linux | Windows |
| --- | --- | --- | --- |
| Values | Keychain, through `/usr/bin/security` | `~/.config/hop/<account>.secrets`, mode 600 | `~/.config/hop/<account>.secrets` |
| Unlock | Touch ID, or your password | your password, checked by `su` | Windows Hello |
| Clipboard | hidden from Maccy and friends | hidden from KDE Klipper and friends | out of history and cloud sync |

The Keychain is only reached through `/usr/bin/security`, the tool hop
stores items with: no password prompt after an update, no "allow access"
window.

Touch ID guards the screen, not the data. Any program running as you can
read the same items, the way hop does. hopla keeps values off your screen
and out of your clipboard history; it is not a password manager.

## Meet Lapla

The bank's bunny watches from the corner of the palette, monocle on.

| He... | when |
| --- | --- |
| hops and says *done !* | something worked |
| says *oops* | it did not |
| closes his eyes, *not looking* | you type a value |
| asks *sure ?* | you are about to delete |
| falls asleep, *z z z* | nothing happens for 20 s |
| dances | you type `hopla` |

## Build

```sh
git clone https://github.com/mirkobozzetto/hopla && cd hopla
cargo build --release
```

Releases are a tag away: bump `version` in `Cargo.toml`, then
`git tag vX.Y.Z && git push --tags`. CI builds macOS (Apple Silicon and
Intel), Linux (x86 and ARM) and Windows, attaches them to the release and
publishes the crate.

## License

MIT
