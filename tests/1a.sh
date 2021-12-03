#!/bin/sh

. ./test-lib.sh

test_expect_success 'init creates HEAD file' 'test -f .gnew/HEAD'
