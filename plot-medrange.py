#!/usr/bin/env python3
from argparse import ArgumentDefaultsHelpFormatter, FileType, ArgumentParser
import json
import sys

from scipy.stats import scoreatpercentile as percentile
import matplotlib
matplotlib.use('Agg')
import pylab as plt  # noqa

plt.rcParams.update({
    'axes.grid': True,
})


def log(*a, **kw):
    print(*a, **kw, file=sys.stderr)


def data_sets_from_input(fd):
    for line in fd:
        yield json.loads(line)


def make_counts(roll_events):
    counts = {i: 0 for i in range(2, 12+1)}
    hards = {4: 0, 6: 0, 8: 0, 10: 0}
    for e in roll_events:
        counts[e.value] += 1
        if e.value in {4, 6, 8, 10} and e.dice[0] == e.dice[1]:
            hards[e.value] += 1
    return counts, hards


def make_expected_counts(num_rolls):
    counts = {
        2: num_rolls * 1 / 36,
        3: num_rolls * 2 / 36,
        4: num_rolls * 3 / 36,
        5: num_rolls * 4 / 36,
        6: num_rolls * 5 / 36,
        7: num_rolls * 6 / 36,
        8: num_rolls * 5 / 36,
        9: num_rolls * 4 / 36,
        10: num_rolls * 3 / 36,
        11: num_rolls * 2 / 36,
        12: num_rolls * 1 / 36,
    }
    hards = {
        4: num_rolls * 1 / 36,
        6: num_rolls * 1 / 36,
        8: num_rolls * 1 / 36,
        10: num_rolls * 1 / 36,
    }
    return counts, hards


def rgb_conv(r, g, b):
    ''' Takes rgb in [0, 255] space and turns into rgb in [0, 1] space '''
    return r / 255, g / 255, b / 255


def plot(
        out_fd, data_sets,
        title='Expected bankroll change over time',
        xlabel='Roll number',
        ylabel='Change in bankroll',
        file_format='png',
        transparent=False):
    ''' Plot the median value across many sets of data, as well as the area
    between the 1st and 3rd quartiles.

    Each input data set should be a dictionary of x,y pairs as specified below.
    All input data sets must have the same x values. At each specified x value,
    plot the median of the y values across all data sets. Also plot the 1st and
    3rd quartiles for the y values at this x value.

    out_fd: the file-like object to which to write the graph in PNG file format
    data_sets: an iterable, containing one or more data set lists

    file_format: file format to output. Must be one of:
        - png
        - svg

    transparent: whether or not the file should have a transparent background

    An example data set dictionary:
        [1, 4, 2, 7, 10]

    Where each value is a y value and the corresponding x value is its position
    '''
    assert file_format in 'png svg svgz'.split(' ')
    plt.figure()
    d = {}
    for data_set in data_sets:
        for x, y in enumerate(data_set):
            if x not in d:
                d[x] = []
            d[x].append(y)
    stats_d = {}
    for x in d:
        stats_d[x] = (
            min(d[x]),
            percentile(d[x], 5),
            percentile(d[x], 25),
            percentile(d[x], 50),
            percentile(d[x], 75),
            percentile(d[x], 95),
            max(d[x]),
        )
    # colors selected to be good for colorblind people
    # http://www.somersault1824.com/tips-for-designing-scientific-figures-for-color-blind-readers/
    # http://mkweb.bcgsc.ca/biovis2012/
    # http://mkweb.bcgsc.ca/colorblind/
    dark_purple = rgb_conv(73, 0, 146)
    dark_blue = rgb_conv(0, 109, 219)
    purple = rgb_conv(182, 109, 255)
    blue = rgb_conv(109, 182, 255)
    light_blue = rgb_conv(182, 219, 255)
    uppest_color = *dark_purple, 0.9
    upper_color = *dark_blue, 0.9
    med_color = *purple, 1
    middle_color = *purple, 0.5
    lower_color = *blue, 0.9
    lowest_color = *light_blue, 0.9
    per_0 = [v[0] for v in stats_d.values()]
    per_5 = [v[1] for v in stats_d.values()]
    per_25 = [v[2] for v in stats_d.values()]
    per_50 = [v[3] for v in stats_d.values()]
    per_75 = [v[4] for v in stats_d.values()]
    per_95 = [v[5] for v in stats_d.values()]
    per_100 = [v[6] for v in stats_d.values()]
    # plt.plot(stats_d.keys(), per_100, color=max_color, label='max')
    plt.plot(stats_d.keys(), per_50, color=med_color, label='median')
    # plt.plot(stats_d.keys(), per_0, color=min_color, label='min')
    plt.fill_between(
        stats_d.keys(), per_100, per_95, color=uppest_color, label='top 5%')
    plt.fill_between(
        stats_d.keys(), per_95, per_75, color=upper_color, label='next 20%')
    plt.fill_between(
        stats_d.keys(), per_75, per_25, color=middle_color, label='middle 50%')
    plt.fill_between(
        stats_d.keys(), per_25, per_5, color=lower_color, label='next 20%')
    plt.fill_between(
        stats_d.keys(), per_5, per_0, color=lowest_color, label='bottom 5%')
    plt.xlim(left=0, right=max(stats_d.keys()))
    ymag = max(max(per_100), -1 * min(per_0))
    plt.ylim(top=ymag, bottom=0)
    plt.xlabel(xlabel)
    plt.ylabel(ylabel)
    plt.legend(loc='best', fontsize=8)
    plt.title(title)
    plt.savefig(out_fd, transparent=transparent, format=file_format)
    log('Median game min =', min(per_50),
        '(loss %d)' % (per_50[0] - min(per_50),))
    log(
        'Median game end =', per_50[-1],
        '(loss %d)' % (per_50[0] - per_50[-1],))


def gen_parser():
    d = 'Plot the median value across many sets of data, as well as the area '\
        'between the 1st and 3rd quartiles'
    p = ArgumentParser(
        formatter_class=ArgumentDefaultsHelpFormatter,
        description=d)
    p.add_argument(
        '-i', '--input', type=FileType('rt'), default=sys.stdin,
        help='From where to read a stream of dictionaries, one per line')
    p.add_argument(
        '-o', '--output', type=FileType('wb'), default='/dev/stdout',
        help='File to which to write graph')
    p.add_argument(
        '--format', choices='png svg svgz'.split(), default='png',
        help='File format to use')
    p.add_argument(
        '--title', type=str, help='Title of graph',
        default='Expected bankroll change over time')
    p.add_argument(
        '--xlabel', type=str, help='Label of X axis', default='Roll number')
    p.add_argument(
        '--ylabel', type=str, help='Label of y axis',
        default='Change in bankroll')
    p.add_argument(
        '--transparent', action='store_true')
    return p


def main(args):
    plot(
        args.output, data_sets_from_input(args.input),
        title=args.title,
        xlabel=args.xlabel,
        ylabel=args.ylabel,
        file_format=args.format,
        transparent=args.transparent,
    )
    return 0


if __name__ == '__main__':
    args = gen_parser().parse_args()
    exit(main(args))
