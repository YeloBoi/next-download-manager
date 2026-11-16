# Security

Do not post tokens, signed URLs, cookies, databases or part manifests in public
issues. If a repository security-reporting channel is enabled, use it privately;
otherwise contact the owner through a private channel before disclosing details.
There is no published security contact identity yet.

Only HTTP(S) is accepted. TLS certificate checks remain enabled. Native messaging
does not accept commands or arbitrary output paths. Downloads never execute
automatically. Local SQLite state and manifests contain full source URLs and are
not encrypted; access is controlled by the current Windows user's filesystem.

This is an unaudited alpha. Confirm hashes from an independent trusted publisher
when available. Keep Windows security features enabled.
