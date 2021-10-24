#!/usr/bin/python3
import sys
import json

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
            with open(inst_filename) as f_inst:
                inst_data = json.load(f_inst)
            inst_data["coloring"] = sol_data
            with open(inst_filename, 'w') as f_inst:
                json.dump(inst_data, f_inst)