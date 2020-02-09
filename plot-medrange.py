#!/usr/bin/env python3
from argparse import ArgumentDefaultsHelpFormatter, FileType, ArgumentParser
import json
import sys

import matplotlib
matplotlib.use('Agg')
import pylab as plt  # noqa

plt.rcParams.update({
    'axes.grid': True,
})


def log(*a, **kw):
    print(*a, **kw, file=sys.stderr)


def ptiles_from_input2(fd):
    for line in fd:
        xval, ptiles = json.loads(line)
        assert len(ptiles) == 7
        yield xval, ptiles


def offset_ptiles2(ptiles, by):
    for xval, ptile in ptiles:
        yield xval, [val - by for val in ptile]


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


def plot2(
        out_fd, ptiles,
        title='Expected bankroll change over time',
        xlabel='Roll number',
        ylabel='Change in bankroll',
        file_format='png',
        transparent=False):
    ''' Plot the median value across many sets of data, as well as the area
    between the 1st and 3rd quartiles.

    The ptiles argument should generate lists, each of length 7. The items are
        1. min at this x,
        2. 5% percentile at this x,
        3. 25% percentile at this x,
        4. median at this x,
        5. 75% percentile at this x,
        6. 95% percentile at this x,
        7. max at this x
    Where the X value is implicit by the order of the lists

    out_fd: the file-like object to which to write the graph in PNG file format
    data_sets: an iterable, containing one or more data set lists

    file_format: file format to output. Must be one of:
        - png
        - svg

    transparent: whether or not the file should have a transparent background
    '''
    assert file_format in 'png svg svgz'.split(' ')
    plt.figure()
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
    p0, p5, p25, p50, p75, p95, p100 = [], [], [], [], [], [], []
    last_idx = -1
    for xval, vals in ptiles:
        if xval > last_idx:
            p0.extend([0] * (xval - last_idx))
            p5.extend([0] * (xval - last_idx))
            p25.extend([0] * (xval - last_idx))
            p50.extend([0] * (xval - last_idx))
            p75.extend([0] * (xval - last_idx))
            p95.extend([0] * (xval - last_idx))
            p100.extend([0] * (xval - last_idx))
            last_idx = len(p0) - 1
        p0[xval] = vals[0]
        p5[xval] = vals[1]
        p25[xval] = vals[2]
        p50[xval] = vals[3]
        p75[xval] = vals[4]
        p95[xval] = vals[5]
        p100[xval] = vals[6]
    x = list(range(len(p0)))
    # plt.plot(stats_d.keys(), per_100, color=max_color, label='max')
    plt.plot(x, p50, color=med_color, label='median')
    # plt.plot(stats_d.keys(), per_0, color=min_color, label='min')
    plt.fill_between(
        x, p100, p95, color=uppest_color, label='top 5%')
    plt.fill_between(
        x, p95, p75, color=upper_color, label='next 20%')
    plt.fill_between(
        x, p75, p25, color=middle_color, label='middle 50%')
    plt.fill_between(
        x, p25, p5, color=lower_color, label='next 20%')
    plt.fill_between(
        x, p5, p0, color=lowest_color, label='bottom 5%')
    plt.xlim(left=0, right=len(p100))
    # ymag = max(max(p100), -1 * min(p0))
    # plt.ylim(top=ymag, bottom=-1*ymag)
    plt.xlabel(xlabel)
    plt.ylabel(ylabel)
    plt.legend(loc='best', fontsize=8)
    plt.title(title)
    plt.savefig(out_fd, transparent=transparent, format=file_format)
    log('Median game min =', min(p50),
        '(loss %d)' % (p50[0] - min(p50),))
    log(
        'Median game end =', p50[-1],
        '(loss %d)' % (p50[0] - p50[-1],))


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
    p.add_argument(
        '--offset-y', type=float, default=0,
        help='Subtract this from all y values')
    p.add_argument('--ymin', type=float)
    p.add_argument('--ymax', type=float)
    return p


def main(args):
    plot2(
        args.output,
        offset_ptiles2(ptiles_from_input2(args.input), args.offset_y),
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
