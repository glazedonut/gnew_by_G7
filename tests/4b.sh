#!/bin/sh

. ./test-lib.sh

test_expect_success 'remove does not remove nonexisting files from tracklist' '
	! gnew remove test.txt 2>out &&
	grep "fatal: file not found" out
'
