#!/bin/sh

. ./test-lib.sh

test_expect_success 'merge with conflicts' '
	test_commit init foo &&
	gnew checkout -b branch1 &&
	echo "hello from branch1" >>foo &&
	gnew commit foo &&
	gnew checkout main &&
	echo "hello from main" >>foo &&
	gnew commit foo &&
	! gnew merge branch1 2>out &&
	grep "Merge conflict in foo" out &&
	cat >expect <<-\EOF &&
	init
	<<<<<<< ours
	hello from main
	||||||| original
	=======
	hello from branch1
	>>>>>>> theirs
	EOF
	diff expect foo
'
