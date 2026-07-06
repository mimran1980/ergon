# CI monitoring — check latest CI run status
ci-status limit='3':
    ./ci-monitor.sh {{ limit }}

# List available recipes
default:
    @just --list
