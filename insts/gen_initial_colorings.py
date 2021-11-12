#!/usr/bin/python3
import sys
import json
from cgshop2022utils.verify import verify_coloring
from cgshop2022utils.io import read_instance, read_solution


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: {} SOLUTION_FILE")
    else:
        sol_filename = sys.argv[1]
        with open(sol_filename) as f:
            sol_data = json.load(f)
            inst_name = sol_data["instance"]
            inst_filename = "insts/cgshop22/{}.instance.json".format(inst_name)
            inst_data = {}
            inst_solution = {}
            with open(inst_filename) as f_inst:
                inst_data = json.load(f_inst)
                inst_solution = inst_data["coloring"]
            # export cgshop inst type
            inst_data["type"] = "Instance_CGSHOP2022"
            with open(inst_filename, "w") as f_inst:
                json.dump(inst_data, f_inst)
            # check solutions
            instance = read_instance(inst_filename)
            error, num_colors = verify_coloring(instance,inst_solution["colors"],expected_num_colors=inst_solution['num_colors'])
            if error != None:# solution is valid and uses exactly num_colors distinct colors
                inst_solution["num_colors"] = 999999999
            # check new solution
            error, num_colors = verify_coloring(
                instance,
                sol_data["colors"],
                expected_num_colors=sol_data['num_colors']
            )
            if not error is None:
                print("{}\t invalid solution".format(inst_solution["instance"]))
            else:
                if inst_solution == {} or inst_solution["num_colors"] > sol_data["num_colors"]:
                    print("{}: {} -> {}".format(inst_solution["instance"], inst_solution["num_colors"], sol_data["num_colors"]))
                    inst_data["coloring"] = sol_data
                    with open(inst_filename, 'w') as f_inst:
                        json.dump(inst_data, f_inst)