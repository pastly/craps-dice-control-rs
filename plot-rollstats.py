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


def counts_from_input(fd):
    for line in fd:
        yield json.loads(line)


def ptiles_from_input(fd):
    for line in fd:
        xval, ptiles = json.loads(line)
        assert len(ptiles) == 7
        yield xval, ptiles


def make_expected_counts(num_rolls):
    return [
        num_rolls * 1 / 36,
        num_rolls * 2 / 36,
        num_rolls * 3 / 36,
        num_rolls * 4 / 36,
        num_rolls * 5 / 36,
        num_rolls * 6 / 36,
        num_rolls * 5 / 36,
        num_rolls * 4 / 36,
        num_rolls * 3 / 36,
        num_rolls * 2 / 36,
        num_rolls * 1 / 36,
    ]


def plot(
        out_fd, input,
        title,
        xlabel,
        ylabel,
        file_format,
        transparent):
    ''' Plot roll freqencies

    The input is a list of dictionaries, one per line. The dicts should have
    the keys 'all' and 'hard'; the latter is ignored. 'all' snould contain a
    list of 11 values that are counts of the occurancies of each possible roll
    value (2-12), in order.

    out_fd: the file-like object to which to write the graph in PNG file format
    data_sets: an iterable, containing one or more data set lists

    file_format: file format to output. Must be one of:
        - png
        - svg

    transparent: whether or not the file should have a transparent background
    '''
    assert file_format in 'png svg svgz'.split(' ')
    counts = [0] * 11
    for cnt in input:
        cnt = cnt['all']
        assert len(counts) == len(cnt)
        for i in range(len(counts)):
            counts[i] += cnt[i]
    total = sum(counts)
    freqs = [c/total for c in counts]
    ymin = 0
    ymax = max(freqs) * 1.1
    expected = make_expected_counts(1)
    plt.figure()
    plt.plot(range(2, 12+1), freqs, label='actual')
    plt.plot(range(2, 12+1), expected, label='expected')
    plt.xlim(left=2, right=12)
    plt.ylim(bottom=ymin, top=ymax)
    plt.xlabel(xlabel)
    plt.ylabel(ylabel)
    plt.legend(loc='best', fontsize=8)
    plt.title(title)
    plt.savefig(out_fd, transparent=transparent, format=file_format)


def gen_parser():
    d = 'Take rollstats and plot them'
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
        default='Rolls')
    p.add_argument(
        '--xlabel', type=str, help='Label of X axis', default='Roll value')
    p.add_argument(
        '--ylabel', type=str, help='Label of y axis',
        default='Frequency')
    p.add_argument(
        '--transparent', action='store_true')
    return p


def main(args):
    plot(
        args.output,
        counts_from_input(args.input),
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
