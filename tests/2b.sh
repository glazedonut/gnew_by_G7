#!/bin/sh

. ./test-lib.sh

test_expect_success 'add does not add nonexisting files to tracklist' '
	! gnew add test.txt 2>out &&
	grep "fatal: file not found" out
'
