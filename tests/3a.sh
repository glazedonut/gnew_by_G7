#!/bin/sh

. ./test-lib.sh


test_expect_success 'status shows added file' '
	echo foo > foo &&
	gnew add foo &&
	gnew status >out &&
	grep "A foo" out
'

test_expect_success 'cat works' '
	gnew commit "add foo" &&
	gnew cat HEAD foo >foo2 &&
	diff foo foo2
'

test_expect_success 'log shows commit author and message' '
	gnew log >out &&
	grep G7 out &&
	grep "add foo" out
'

test_expect_success 'heads shows current branch' '
	gnew heads >out &&
	grep "* main" out
'
