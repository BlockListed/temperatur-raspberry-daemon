import random

#raise FileNotFoundError

if random.randint(0, 1) == 1:
    raise FileNotFoundError

print(f"{round(random.uniform(10, 40), 2)}")