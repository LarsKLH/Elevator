#! /usr/bin/bash

sleep_time_read=0.5
sleep_time_write=0.8

polls_before_dead=4

counting_file=/home/gia2/yuuuuge_programs/yuge_stud/sanntidsprogramering/git_repo_dir/TTK4515-2025-Gruppe26/exercises/Exercise4-master/counting_file.txt

program_file=/home/gia2/yuuuuge_programs/yuge_stud/sanntidsprogramering/git_repo_dir/TTK4515-2025-Gruppe26/exercises/Exercise4-master/ex4_program_c.sh

terminal_profile=sanntidsprogramering_exercise_4_terminal_profile

prev_number="0"

changeless_checks=0

is_counter=(false)

while true;
do
	if $is_counter; then
		((prev_number++))
		
		echo $prev_number >> $counting_file

		echo -e "Wrote $prev_number to the counting file \n"

		sleep $sleep_time_write
	else
		latest_number=$(tail -n 1 $counting_file)

		if [ "$prev_number" != "$latest_number" ]; then
			echo -e "New number; prev: $prev_number, new: $latest_number \n"

			prev_number=$latest_number
			changeless_checks=0
		else
			((changeless_checks++))
			echo "No new number for the past $changeless_checks checks"
		fi

		if [ $changeless_checks -gt $polls_before_dead ]; then
			echo -e "Assuming the counter is dead spawning new and taking control\n"

			gnome-terminal --window-with-profile=$terminal_profile -- $program_file
			is_counter=(true)
		else
			sleep $sleep_time_write
		fi
	fi

done
