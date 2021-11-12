#!/usr/bin/python3
import json
import csv

inst_csv_filename = "insts/cgshop22.csv"

# TODO generate CSV lines + folder with solutions
with open(inst_csv_filename) as f:
    reader = csv.DictReader(f)
    for row in reader:
        inst_filename = "insts/"+row["path"]
        # print(inst_filename)
        with open(inst_filename) as f_inst:
            inst_data = json.load(f_inst)
            print(inst_data["coloring"]["num_colors"])
            sol_filename = "experiments/bk/"+row["name"]+".solution.json"
            with open(sol_filename, 'w') as outfile:
                json.dump(inst_data["coloring"], outfile)
            
        # json_filename = "26_09_21_rwls_clique/{}.instance.json.output.json".format(row["name"])
        # bound = -1
        # with open(json_filename) as json_file:
        #     content = "".join(json_file.readlines()).strip("'<>() ").replace("\n","").replace(" ", "")
        #     if content != "null":
        #         data = json.loads(content)
        #         if "Bound" in data and "Value" in data["Bound"]:
        #             bound = int(data["Bound"]["Value"])
        # print("{},{}".format(row["name"], bound))

