use crate::model::QualityLocation;

pub(crate) fn line_location(line: usize, line_len: usize) -> QualityLocation {
    QualityLocation {
        start_line: line,
        start_column: 1,
        end_line: line,
        end_column: line_len.max(1),
    }
}

pub(crate) fn region_location(
    start_line: usize,
    start_len: usize,
    end_line: usize,
    end_len: usize,
) -> QualityLocation {
    QualityLocation {
        start_line,
        start_column: 1,
        end_line,
        end_column: if end_line == start_line {
            start_len.max(end_len).max(1)
        } else {
            end_len.max(1)
        },
    }
}
