use nu_protocol::{IntoPipelineData, Span, Value};

#[test]
fn test_convert_pipeline_data_to_value() {
    // Setup PipelineData
    let value_val = 10;
    let value = Value::int(value_val, Span::new(1, 3));
    let pipeline_data = value.into_pipeline_data();

    // Test that conversion into Value is correct
    let new_span = Span::new(5, 6);
    let converted_value = pipeline_data.into_value(new_span);

    assert_eq!(converted_value, Ok(Value::int(value_val, new_span)));
}

#[test]
fn constructors_preserve_values_and_metadata() {
    use nu_protocol::{PipelineData, PipelineMetadata};
    let span = Span::new(1, 3);
    let metadata = PipelineMetadata::default().with_content_type(Some("application/test".into()));
    let empty = PipelineData::empty();
    assert!(matches!(empty, PipelineData::Empty));
    assert_eq!(empty.metadata(), None);
    assert_eq!(empty.span(), None);
    let data = PipelineData::value(Value::int(42, span), Some(metadata.clone()));
    assert_eq!(data.metadata(), Some(metadata.clone()));
    assert_eq!(data.into_value(span).unwrap(), Value::int(42, span));
    // Exercise the migrated IntoPipelineData implementation as well.
    let data = Value::int(42, span).into_pipeline_data_with_metadata(metadata.clone());
    assert_eq!(data.metadata(), Some(metadata));
    assert_eq!(data.into_value(span).unwrap(), Value::int(42, span));
}

#[test]
fn constructors_keep_streams_lazy_and_metadata_intact() {
    use nu_protocol::{
        ByteStream, ByteStreamType, ListStream, PipelineData, PipelineMetadata, Signals,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let span = Span::new(1, 3);
    let metadata = PipelineMetadata::default().with_content_type(Some("application/test".into()));
    let reads = Arc::new(AtomicUsize::new(0));
    let observed = reads.clone();
    let stream = ListStream::new(
        (0..2).map(move |n| {
            observed.fetch_add(1, Ordering::SeqCst);
            Value::int(n, span)
        }),
        span,
        Signals::empty(),
    );
    let data = PipelineData::list_stream(stream, Some(metadata.clone()));
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    assert_eq!(data.metadata(), Some(metadata.clone()));
    assert_eq!(data.span(), Some(span));
    assert_eq!(data.into_iter().count(), 2);
    assert_eq!(reads.load(Ordering::SeqCst), 2);

    let reads = Arc::new(AtomicUsize::new(0));
    let observed = reads.clone();
    let mut stream =
        ByteStream::from_fn(span, Signals::empty(), ByteStreamType::Binary, move |buf| {
            if observed.fetch_add(1, Ordering::SeqCst) == 0 {
                buf.extend_from_slice(b"abc");
                Ok(true)
            } else {
                Ok(false)
            }
        })
        .with_known_size(Some(3));
    stream.push_caller_span(Span::new(10, 20));
    let data = PipelineData::byte_stream(stream, Some(metadata.clone()));
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    assert_eq!(data.metadata(), Some(metadata));
    let PipelineData::ByteStream(stream, _) = data else {
        panic!("expected byte stream")
    };
    assert_eq!(stream.known_size(), Some(3));
    assert_eq!(stream.type_(), ByteStreamType::Binary);
    assert_eq!(stream.span(), span);
    assert_eq!(stream.get_caller_spans(), &[Span::new(10, 20)]);
    assert_eq!(stream.into_bytes().unwrap(), b"abc");
    assert!(reads.load(Ordering::SeqCst) > 0);
}

#[test]
fn constructors_preserve_stream_interrupt_sources() {
    use nu_protocol::{ByteStream, ByteStreamType, ListStream, PipelineData, Signals};
    use std::sync::{Arc, atomic::AtomicBool};
    let signals = Signals::new(Arc::new(AtomicBool::new(false)));
    let span = Span::test_data();
    let list = PipelineData::list_stream(
        ListStream::new(
            std::iter::repeat_with(|| Value::test_int(1)),
            span,
            signals.clone(),
        ),
        None,
    );
    let bytes = PipelineData::byte_stream(
        ByteStream::from_iter(
            [b"abc".as_slice()],
            span,
            signals.clone(),
            ByteStreamType::Binary,
        ),
        None,
    );
    signals.trigger();
    assert_eq!(list.into_iter().count(), 0);
    let PipelineData::ByteStream(stream, _) = bytes else {
        panic!("expected byte stream")
    };
    assert_eq!(stream.chunks().unwrap().count(), 0);
}

#[test]
fn stream_into_conversions_preserve_lazy_streams_without_metadata() {
    use nu_protocol::{ByteStream, ByteStreamType, ListStream, PipelineData, Signals};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let span = Span::new(10, 20);
    let reads = Arc::new(AtomicUsize::new(0));
    let observed = reads.clone();
    let list = ListStream::new(
        (0..2).map(move |n| {
            observed.fetch_add(1, Ordering::SeqCst);
            Value::int(n, span)
        }),
        span,
        Signals::empty(),
    );
    let data: PipelineData = list.into();
    assert_eq!(data.metadata(), None);
    assert_eq!(data.span(), Some(span));
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    assert_eq!(
        data.into_value(span).unwrap(),
        Value::list(vec![Value::int(0, span), Value::int(1, span)], span)
    );
    assert_eq!(reads.load(Ordering::SeqCst), 2);

    let reads = Arc::new(AtomicUsize::new(0));
    let observed = reads.clone();
    let bytes = ByteStream::from_iter(
        std::iter::once_with(move || {
            observed.fetch_add(1, Ordering::SeqCst);
            b"abc".to_vec()
        }),
        span,
        Signals::empty(),
        ByteStreamType::Binary,
    )
    .with_known_size(Some(3));
    let data = PipelineData::from(bytes);
    assert_eq!(data.metadata(), None);
    assert_eq!(reads.load(Ordering::SeqCst), 0);
    let PipelineData::ByteStream(stream, _) = data else {
        panic!("expected byte stream")
    };
    assert_eq!(stream.known_size(), Some(3));
    assert_eq!(stream.type_(), ByteStreamType::Binary);
    assert_eq!(stream.span(), span);
    assert_eq!(stream.into_bytes().unwrap(), b"abc");
    assert_eq!(reads.load(Ordering::SeqCst), 1);
}
