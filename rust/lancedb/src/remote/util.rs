use std::io::Cursor;

use arrow_array::RecordBatchReader;

use crate::Result;

pub fn batches_to_ipc_bytes(batches: impl RecordBatchReader) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    {
        let mut cursor = Cursor::new(&mut buf);
        let mut writer = arrow_ipc::writer::StreamWriter::try_new(&mut cursor, &batches.schema())?;
        for batch in batches {
            let batch = batch?;
            writer.write(&batch)?;
        }
        writer.finish()?;
    }

    Ok(buf)
}
