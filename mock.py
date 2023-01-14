import random
import sys

if random.randint(0, 1) == 1:
    sys.exit(1)

print(f"{random.randint(350, 1200)},{round(random.uniform(10, 40), 2)}")