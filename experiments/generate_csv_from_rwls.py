import json
import csv

inst_csv_filename = "../insts/cgshop22.csv"

with open(inst_csv_filename) as f:
    reader = csv.DictReader(f)
    for row in reader:
        json_filename = "26_09_21_rwls_clique/{}.instance.json.output.json".format(row["name"])
        bound = -1
        with open(json_filename) as json_file:
            content = "".join(json_file.readlines()).strip("'<>() ").replace("\n","").replace(" ", "")
            if content != "null":
                data = json.loads(content)
                if "Bound" in data and "Value" in data["Bound"]:
                    bound = int(data["Bound"]["Value"])
        print("{},{}".format(row["name"], bound))

