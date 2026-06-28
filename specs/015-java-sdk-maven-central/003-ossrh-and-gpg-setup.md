# 003 - OSSRH and GPG Setup

## A) Create OSSRH credentials

From Sonatype Central:

1. Create a publishing token
2. Save token username and token secret
3. Store securely; these are CI release credentials

## B) Generate a dedicated release GPG key

```bash
gpg --full-generate-key
gpg --list-secret-keys --keyid-format LONG
```

Choose a modern key type and strong passphrase.

## C) Export private key (ASCII armored)

```bash
gpg --armor --export-secret-keys <KEY_ID>
```

Copy the full block from:

- `-----BEGIN PGP PRIVATE KEY BLOCK-----`
- to `-----END PGP PRIVATE KEY BLOCK-----`

## D) Validate local signing quickly

```bash
echo "sign-check" > /tmp/sign-check.txt
gpg --armor --detach-sign /tmp/sign-check.txt
```

If this works, your key + passphrase are valid locally.

## E) Required GitHub secrets

Add these repository secrets:

- `OSSRH_USERNAME`
- `OSSRH_TOKEN`
- `OSSRH_GPG_SECRET_KEY`
- `OSSRH_GPG_SECRET_KEY_PASSWORD`

Notes:

- `OSSRH_GPG_SECRET_KEY` must be the full armored private key text
- `OSSRH_GPG_SECRET_KEY_PASSWORD` is the passphrase used for that key

