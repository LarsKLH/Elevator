#! /usr/bin/bash

echo 0 > /home/gia2/yuuuuge_programs/yuge_stud/sanntidsprogramering/git_repo_dir/TTK4515-2025-Gruppe26/exercises/Exercise4-master/counting_file.txt

gnome-terminal --window-with-profile=sanntidsprogramering_exercise_4_terminal_profile -- /home/gia2/yuuuuge_programs/yuge_stud/sanntidsprogramering/git_repo_dir/TTK4515-2025-Gruppe26/exercises/Exercise4-master/ex4_program_a.sh


for i in `seq 1 10`;
	do
                echo $i >> counting_file.txt
		sleep 1
        done




