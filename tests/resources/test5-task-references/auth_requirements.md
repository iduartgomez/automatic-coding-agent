# Authentication Requirements

## Features Needed
- JWT token-based authentication
- Password hashing with bcrypt
- Session management
- Role-based access control (RBAC)

## Technical Specifications
- Use FastAPI for endpoints
- PostgreSQL for user storage
- Redis for session caching
- Add rate limiting for login attempts

## API Endpoints Required
- POST /auth/login
- POST /auth/register
- POST /auth/logout
- GET /auth/profile
- PUT /auth/profile

## Security Requirements
- Passwords must be hashed with bcrypt (cost factor 12)
- JWT tokens expire after 24 hours
- Refresh tokens expire after 30 days
- Rate limiting: 5 login attempts per minute per IP
- Session invalidation on password change
- Secure cookie settings for production

## Database Schema
- Users table with id, username, email, password_hash, role, created_at
- Sessions table for token management
- Audit log for security events