# Authentication System Implementation

Implement a secure user authentication system for the web application with the following requirements:

## Core Features
- JWT token-based authentication
- Password hashing using bcrypt
- Session management with Redis
- Role-based access control (RBAC)

## Technical Requirements
- Use RS256 for JWT signing
- Implement middleware for route protection
- Add comprehensive error handling
- Support for password reset functionality
- Rate limiting for login attempts

## Security Considerations
- Secure password requirements (min 12 chars, special chars, etc.)
- Protection against timing attacks
- Secure session storage
- CSRF protection

## Testing Requirements
- Unit tests for authentication logic
- Integration tests for API endpoints
- Security testing for common vulnerabilities

Please implement this system following security best practices and ensure all edge cases are properly handled.