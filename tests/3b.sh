#!/bin/sh

. ./test-lib.sh

test_expect_success 'status shows untracked file' '
	echo foo >foo &&
	gnew status >out &&
	grep "? foo" out
'

test_expect_success 'cat produces an error when trying to read nonexisting file' '
	gnew commit "init" >out &&
	gnew cat HEAD test 2>&1 | grep "fatal: file not found"
'

test_expect_success 'log 1 prints exactly one commit' '
	gnew commit "second" &&
	! gnew log 1 | grep init	
'

test_expect_success 'heads shows other branches' '
	gnew heads
'
