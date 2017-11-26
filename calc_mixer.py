def chunks(l, n):
    """Yield successive n-sized chunks from l."""
    for i in range(0, len(l), n):
        yield l[i:i + n]

pulse_table = list(map(lambda n: 0 if n == 0 else 95.52 / (8128.0 / n + 100), range(31)))
tnd_table = list(map(lambda n: 0 if n == 0 else 163.67 / (24329.0 / n + 100), range(203)))

print(pulse_table, len(pulse_table))
print(tnd_table, len(tnd_table))

max = (1 << 16) - 1
pulse_table = list(map(lambda n: "{0:#06x}".format(int(n * max)), pulse_table))
tnd_table = list(map(lambda n: "{0:#06x}".format(int(n * max)), tnd_table))


print(',\n'.join([', '.join(l) for l in chunks(pulse_table, 6)]))
print(',\n'.join([', '.join(l) for l in chunks(tnd_table, 6)]))
