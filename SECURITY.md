# Security Policy

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.** Instead, use GitHub's Private Vulnerability Reporting feature:

1. Go to the [Security tab](../../security) of this repository
2. Click "Report a vulnerability"
3. Fill out the private advisory form with details of the issue

You'll be able to communicate with me privately, and we'll work together on a fix before any public disclosure.

**For non-vulnerability security questions**, you're welcome to open a regular issue or discussion.

## Response Timeline

- **Acknowledgment**: Within 48 hours of report submission
- **Initial assessment**: Within 1 week (confirmation of whether it's valid, severity estimation)
- **Fix and patch release**: Within 4 weeks for high-severity issues; I'll prioritize based on severity and impact. Low-severity issues may take longer.
- **Public disclosure**: I'll give you at least 30 days notice before publicly disclosing a fixed vulnerability, or until the next release is published, whichever comes first

I maintain this project solo as a self-hosted tool for small groups. If a response is delayed, I'll update the private advisory with an ETA.

## Supported Versions

Only the **latest release** receives security patches and updates. Guidance is pre-1.0 and breaking changes are possible between minor versions; upgrading to the latest is the best way to stay secure.

| Version | Status | Support Ends |
|---------|--------|--------------|
| Latest (0.x.x) | Supported | Next minor release |
| Older 0.x.x | EOL | Immediately upon next release |

If you're self-hosting an older version and discover a security issue, please still report it — I can advise on workarounds even if a backport isn't feasible.

## Known Limitations & Security Assumptions

Guidance is a **self-hosted TTRPG application for trusted groups**, not a multi-tenant SaaS platform. Please read the following before running it against sensitive data:

### Authentication & Authorization

- **Single-instance, single-group model**: Guidance assumes everyone with network access to the instance is a trusted member of your gaming group. There is no built-in concept of hostile actors on the same instance.
- **Simple session-based auth**: Guidance uses session cookies for authentication. HTTPS is strongly recommended when running over a network (even locally, if accessed remotely).
- **No audit logging**: There is currently no audit trail of who made what changes to character data. If you need that for your use case, self-host with a proxy that logs HTTP requests.

### Data Storage

- **SQLite on disk**: Character data and imported sourcebooks are stored in a single SQLite file on the host machine. Encrypt the underlying filesystem if you need encryption at rest.
- **No built-in backup**: Guidance does not create backups. You're responsible for backing up the SQLite file (and `.env` / configuration files, if you have them). Regular `cp` or a scheduled backup tool is sufficient.
- **No data validation on import**: User-supplied JSON (sourcebooks) is parsed and stored. Malicious or malformed JSON could cause issues; Guidance assumes you trust the source of imported data.

### Access Control

- **All users see all content**: Once a sourcebook is imported or a character is created, every authenticated user on that instance can see it. There is no per-character or per-campaign access control.
- **Limited role-based restrictions**: There are two permission levels: instance owner (unrestricted) and regular users (whose capabilities can be customized, e.g., import permissions, homebrew creation). However, all users can view all imported content and all created characters. No fine-grained resource-level ACLs (per-character or per-campaign visibility). Consider this when deciding who gets an account.

### Known Issues

None known at this time, but this is early-stage software. Please review [Issues](../../issues) and [Discussions](../../discussions) before deployment to check for any unresolved concerns.

## Security Best Practices for Self-Hosters

1. **Run over HTTPS** — even on a local network, use a reverse proxy (nginx, Caddy) with a self-signed certificate or Let's Encrypt
2. **Firewall your instance** — only expose it to the people who need it; consider a VPN if accessing remotely
3. **Keep Rust/Leptos/dependencies updated** — `cargo update` regularly, or use Dependabot alerts (enabled by default on public repos)
4. **Backup your data** — `*.sqlite` files and `.env` (if present) should be backed up regularly
5. **Audit the code** — this is open source; review it before running it, especially if you have concerns about the implementation

## Contact

- **Security concerns**: Use private vulnerability reporting (link above)
- **General questions**: Open an issue or discussion
- **Large-scale deployments**: I'm happy to advise; open an issue or start a discussion

---

**Last updated**: 2026-07-29  
**Guidance version**: v0.x.x
