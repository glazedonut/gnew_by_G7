#!/bin/sh

. ./test-lib.sh

test_expect_success 'cat works' '
	gnew add foo > foo &&
	gnew add foo &&
	gnew commit "add foo" >c1 &&
	gnew cat $(cat c1) foo >foo2 &&
	diff foo foo2
'
