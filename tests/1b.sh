#!/bin/sh

. ./test-lib.sh

test_expect_success 'init fails if repository already exists' '! gnew init'
