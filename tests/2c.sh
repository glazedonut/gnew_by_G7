#!/bin/sh

. ./test-lib.sh

test_expect_success 'commit correctly creates objects' '
	test_commit initial file &&	
	ls .gnew/objects | wc -l | grep 3
'

test_expect_success 'commit correctly sets the branch head' '
	test_commit initial file >gout &&
	cat .gnew/heads/main > cout &&
	diff gout cout
'
#	test_commit initial file &&
#	cat .gnew/heads/main | wc -l | grep 1
#'
