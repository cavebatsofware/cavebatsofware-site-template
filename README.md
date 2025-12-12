# cavebatsofware-site-template

[![Cargo Check](https://github.com/cavebatsofware/cavebatsofware-site-template/actions/workflows/check.yml/badge.svg)](https://github.com/cavebatsofware/cavebatsofware-site-template/actions/workflows/check.yml)
[![Cargo Format](https://github.com/cavebatsofware/cavebatsofware-site-template/actions/workflows/format.yml/badge.svg)](https://github.com/cavebatsofware/cavebatsofware-site-template/actions/workflows/format.yml)
[![Lint](https://github.com/cavebatsofware/cavebatsofware-site-template/actions/workflows/lint.yml/badge.svg)](https://github.com/cavebatsofware/cavebatsofware-site-template/actions/workflows/lint.yml)
[![Cargo Audit](https://github.com/cavebatsofware/cavebatsofware-site-template/actions/workflows/audit.yml/badge.svg)](https://github.com/cavebatsofware/cavebatsofware-site-template/actions/workflows/audit.yml)

A basic site template built with Rust and Axum featuring:
- Code-gated document access for controlled distribution (e.g., resumes, proposals)
- Admin panel (React SPA) with email verification and MFA support
- Public frontend (Astro SSR, replaceable with your own)
- PostgreSQL database with SeaORM
- Rate limiting and access logging

## Quick Start

### Prerequisites

- Rust (latest stable)
- Docker and Docker Compose
- Node.js and npm
- AWS account with SES configured (for admin email verification)

### Setup

```bash
# Clone and enter the directory
git clone <repo-url>
cd cavebatsofware-site-template

# Create environment configuration
cp .env.example .env
# Edit .env with your values

# Run setup (creates .env if missing, installs npm deps, starts db, runs migrations)
make setup

# Start development server with hot reload
make dev
```

The application runs at `http://localhost:3000`. Run `make help` to see all available commands.

### Endpoints

Public routes:
- `/` - Landing page (served from public frontend)
- `/access/{code}` - Code-gated document page
- `/access/{code}/download` - Download document
- `/document/{code}` - Alias for access page
- `/health` - Health check
- `/api/contact` - Contact form submission
- `/api/subscribe` - Newsletter subscription

Admin routes:
- `/admin` - Admin panel SPA
- `/api/admin/register` - Create admin account
- `/api/admin/login` - Login
- `/api/admin/verify-email` - Email verification (required before login)
- `/api/admin/mfa/*` - MFA setup and verification
- `/api/admin/access-codes` - Manage access codes
- `/api/admin/access-logs` - View access logs
- `/api/admin/settings` - Site settings

### Renaming the Template

To rebrand the template for your project, do a project-wide find/replace of `cavebatsofware-site-template` with your application name. Key files to update:
- `Cargo.toml` (package name)
- `Makefile` (DOCKER_IMAGE)
- `README.md`
- Directory name

## Configuration

Copy `.env.example` to `.env` and configure:

**Required:**
- `DATABASE_URL` - PostgreSQL connection string
- `SITE_DOMAIN` / `SITE_URL` - Your domain for emails and links
- `TOTP_ENCRYPTION_KEY` - For MFA (generate with `openssl rand -hex 32`)

**AWS (required for admin accounts):**
- `AWS_SES_FROM_EMAIL` - Verified SES sender address (admin email verification requires SES; other providers welcome as contributions)
- `S3_BUCKET_NAME` - For document storage

Some values like `SITE_URL` and sender email address can be overridden through the admin settings panel.

See `.env.example` for all options with descriptions.

## Development

The Makefile provides all development commands. Key commands:

```bash
make dev              # Start with hot reload (requires cargo-watch, auto-installed)
make dev-no-watch     # Start without hot reload
make test             # Run tests (starts test database automatically)
make clippy           # Run linter
```

### Database

```bash
make db-up            # Start PostgreSQL
make db-down          # Stop PostgreSQL
make db-migrate       # Run migrations
make db-shell         # Open psql shell
make db-reset         # Reset database (WARNING: deletes data)
make db-backup        # Backup to ./backups/
make db-restore       # Restore from backup
```

### Testing

Tests use a separate database on port 5433 to avoid conflicts:

```bash
make test             # Run all tests
make test-db-up       # Start test database only
make test-db-reset    # Reset test database
```

## Custom Public Frontend

The template includes a default Astro welcome page. To use your own Astro site:

1. Set `PUBLIC_FRONTEND_PATH` in `.env`:
   ```bash
   PUBLIC_FRONTEND_PATH=/path/to/your/astro-site
   ```

2. Your Astro site must:
   - Have `npm run build` script
   - Output to `dist/` (Astro default)
   - Have its own `package.json`

Both `make dev` and `make build` will use your external site automatically.

## Deployment

### Docker Build

```bash
# Build Docker image
make build

# Test locally
make run
```

### ECR Deployment

Configure ECR settings in `.env`:
```bash
ECR_REGISTRY_URL=<account-id>.dkr.ecr.<region>.amazonaws.com
ECR_REPO_NAME=your-repo-name
ECR_REGION=us-east-2
```

Then deploy:
```bash
make check-prereqs    # Verify Docker and AWS CLI setup
make deploy           # Build, tag, and push to ECR
```

### Production Database

Update `DATABASE_URL` in your production environment:
```bash
DATABASE_URL=postgresql://user:password@your-db-host:5432/dbname
```

Run migrations on first deploy:
```bash
MIGRATE_DB=true cargo run -- migrate
```

## Security Features

- **Code-gated access** - Server-side validation of access codes
- **Email verification** - Admin accounts require email verification before login
- **Rate limiting** - Configurable per-minute limits with automatic IP blocking
- **Access logging** - Full audit trail with IP addresses and access codes
- **MFA support** - TOTP-based two-factor for admin accounts
- **CSRF protection** - Token validation on data-changing endpoints

## Project Structure

```
├── src/
│   ├── main.rs          # Entry point and routes
│   ├── admin/           # Admin panel handlers
│   ├── entities/        # SeaORM database entities
│   ├── migration/       # Database migrations
│   └── middleware/      # Rate limiting, auth, etc.
├── admin-frontend/      # React admin SPA source
├── public-frontend/     # Astro public site template
├── admin-assets/        # Built admin frontend
├── public-assets/       # Built public frontend
└── assets/              # Static assets (icons, etc.)
```

## License

This project is dual-licensed under your choice of:

- [BSD-3-Clause](https://opensource.org/licenses/BSD-3-Clause)
- [GPL-3.0-only](https://www.gnu.org/licenses/gpl-3.0.html)
