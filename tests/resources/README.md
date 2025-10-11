# Claude Integration Test Resources

This directory contains minimal test setup files for integration testing. Each test directory contains only the original task files needed to set up the test scenario.

## Test Structure

- **test1-simple-file/**: Basic file creation test
- **test2-readme-creation/**: Documentation generation test
- **test3-file-editing/**: File modification test
- **test4-multi-task/**: Multiple task execution test
- **test5-task-references/**: Task reference functionality test
- **test6-nested-complex-tasks/**: Complex nested task decomposition with linked specifications

## Usage

These resources are used by the integration tests to create isolated temporary workspaces. All generated files are created in temporary directories and automatically cleaned up after tests complete.

**Note**: This directory should only contain the original task setup files. Generated files (Python scripts, JSON configs, session files, etc.) are created in temporary workspaces during test execution.