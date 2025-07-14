// WASI Preview 2 (component model) implementation for Agave OS
// This provides the modern WASI APIs using the component model

use super::error::*;
use super::types::*;
use super::{cli, clocks, filesystem, http, io, random, sockets};
use alloc::string::String;
use alloc::vec::Vec;
use sockets::{IpAddressFamily, IpSocketAddress, Network, TcpSocket};

// Helper function to convert WasiError to u32 ErrorCode
fn wasi_error_to_error_code(_err: WasiError) -> u32 {
    // For now, convert all errors to a generic error code
    // In a full implementation, this would map specific error types
    1u32
}

// Helper function to convert WasiError to StreamError
fn wasi_error_to_stream_error(_err: WasiError) -> StreamError {
    StreamError::Closed
}

// Additional helper functions for various error conversion patterns
fn wasi_error_to_u16(error: WasiError) -> u16 {
    error.errno
}

fn option_u32_result_wasi_to_unit(result: Result<Option<u32>, WasiError>) -> Result<u32, ()> {
    match result {
        Ok(Some(val)) => Ok(val),
        Ok(None) => Err(()),
        Err(_) => Err(()),
    }
}

fn unit_result_wasi_to_unit(result: Result<(), WasiError>) -> Result<(), ()> {
    match result {
        Ok(()) => Ok(()),
        Err(_) => Err(()),
    }
}

fn option_u32_result_convert(result: Result<Option<u32>, WasiError>) -> Result<Option<u32>, u32> {
    match result {
        Ok(val) => Ok(val),
        Err(error) => Err(wasi_error_to_error_code(error)),
    }
}

fn unit_result_convert(result: Result<(), WasiError>) -> Result<(), u32> {
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(wasi_error_to_error_code(error)),
    }
}

// Additional types for Preview 2
pub type Instant = u64;
pub type Datetime = u64;
pub type StreamStatus = u8;
pub type ErrorCode = u32;
pub type Descriptor = u32;
pub type DescriptorFlags = u32;
pub type DescriptorType = u8;
pub type NewTimestamp = u64;
pub type DirectoryEntryStream = u32;
pub type Fields = u32;
pub type PathFlags = u16;
pub type OpenFlags = u16;
pub type DescriptorStat = u64;
pub type MetadataHashValue = u64;

// HTTP types
pub type IncomingRequest = u32;
pub type OutgoingRequest = u32;
pub type IncomingResponse = u32;
pub type OutgoingResponse = u32;
pub type FutureIncomingResponse = u32;
pub type RequestOptions = u32;

// Additional missing types for Preview 2 compatibility
pub type WasiDuration = u64; // Use WasiDuration to avoid conflict with core::time::Duration
pub type Timestamp = u64;
pub type Pollable = u32;
pub type InputStream = u32;
pub type OutputStream = u32;
pub type ShutdownType = u8;
pub type TerminalInput = u32;
pub type TerminalOutput = u32;
pub type HeaderError = super::error::WasiError;
pub type IncomingStream = u32;
pub type OutgoingStream = u32;
pub type Trailers = u32;
pub type Method = String;
pub type Scheme = String;
pub type Headers = u32;
pub type ResponseOutparam = u32;
pub type StatusCode = u16;

// Component model exports
pub struct Component;

impl Component {
    pub fn new() -> Self {
        Self
    }

    // WIT bindings for wasi:clocks/monotonic-clock@0.2.0
    pub fn monotonic_clock_now(&self) -> Result<Instant, WasiError> {
        clocks::monotonic_now()
    }

    pub fn monotonic_clock_resolution(&self) -> Result<WasiDuration, WasiError> {
        // Convert u64 to WasiDuration
        clocks::monotonic_resolution().map(|res| res as WasiDuration)
    }

    pub fn monotonic_clock_subscribe_instant(&self, when: Instant) -> Result<Pollable, WasiError> {
        clocks::subscribe_instant(when)
    }

    pub fn monotonic_clock_subscribe_duration(
        &self,
        when: WasiDuration,
    ) -> Result<Pollable, WasiError> {
        Ok(clocks::subscribe_duration(when))
    }

    // WIT bindings for wasi:clocks/wall-clock@0.2.0
    pub fn wall_clock_now(&self) -> Result<Datetime, WasiError> {
        clocks::wall_now()
    }

    pub fn wall_clock_resolution(&self) -> Result<Datetime, WasiError> {
        clocks::wall_resolution()
    }

    // WIT bindings for wasi:random/random@0.2.0
    pub fn random_get_random_bytes(&self, len: u64) -> Result<Vec<u8>, WasiError> {
        random::get_random_bytes(len)
    }

    pub fn random_get_random_u64(&self) -> Result<u64, WasiError> {
        random::get_random_u64()
    }

    pub fn random_insecure_random(&self) -> Result<(u64, u64), WasiError> {
        // Convert single u64 to tuple by splitting bits or generating twice
        random::insecure_random().map(|val| (val, val))
    }

    pub fn random_insecure_random_bytes(&self, len: u64) -> Result<Vec<u8>, WasiError> {
        random::insecure_random_bytes(len)
    }

    // WIT bindings for wasi:io/streams@0.2.0
    pub fn io_read(
        &self,
        this: InputStream,
        len: u64,
    ) -> Result<(Vec<u8>, StreamStatus), StreamError> {
        io::read(this, len).map_err(wasi_error_to_stream_error)
    }

    pub fn io_blocking_read(
        &self,
        this: InputStream,
        len: u64,
    ) -> Result<(Vec<u8>, StreamStatus), StreamError> {
        io::blocking_read(this, len).map_err(wasi_error_to_stream_error)
    }

    pub fn io_skip(&self, this: InputStream, len: u64) -> Result<(u64, StreamStatus), StreamError> {
        io::skip(this, len).map_err(wasi_error_to_stream_error)
    }

    pub fn io_blocking_skip(
        &self,
        this: InputStream,
        len: u64,
    ) -> Result<(u64, StreamStatus), StreamError> {
        io::blocking_skip(this, len).map_err(wasi_error_to_stream_error)
    }

    pub fn io_subscribe(&self, this: InputStream) -> Pollable {
        io::subscribe_to_input_stream(this)
    }

    pub fn io_drop_input_stream(&self, this: InputStream) {
        io::drop_input_stream(this)
    }

    pub fn io_check_write(&self, this: OutputStream) -> Result<u64, StreamError> {
        io::check_write(this).map_err(wasi_error_to_stream_error)
    }

    pub fn io_write(&self, this: OutputStream, contents: Vec<u8>) -> Result<(), StreamError> {
        io::write(this, &contents).map_err(wasi_error_to_stream_error)
    }

    pub fn io_blocking_write_and_flush(
        &self,
        this: OutputStream,
        contents: Vec<u8>,
    ) -> Result<(), StreamError> {
        io::blocking_write_and_flush(this, &contents).map_err(wasi_error_to_stream_error)
    }

    pub fn io_flush(&self, this: OutputStream) -> Result<(), StreamError> {
        io::flush(this).map_err(wasi_error_to_stream_error)
    }

    pub fn io_blocking_flush(&self, this: OutputStream) -> Result<(), StreamError> {
        io::blocking_flush(this).map_err(wasi_error_to_stream_error)
    }

    pub fn io_subscribe_output(&self, this: OutputStream) -> Pollable {
        io::subscribe_to_output_stream(this)
    }

    pub fn io_drop_output_stream(&self, this: OutputStream) {
        io::drop_output_stream(this)
    }

    // WIT bindings for wasi:filesystem/types@0.2.0
    pub fn filesystem_read_via_stream(
        &self,
        this: Descriptor,
        offset: FileSize,
    ) -> Result<InputStream, ErrorCode> {
        filesystem::read_via_stream(this, offset).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_write_via_stream(
        &self,
        this: Descriptor,
        offset: FileSize,
    ) -> Result<OutputStream, ErrorCode> {
        filesystem::write_via_stream(this, offset).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_append_via_stream(
        &self,
        this: Descriptor,
    ) -> Result<OutputStream, ErrorCode> {
        filesystem::append_via_stream(this).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_advise(
        &self,
        this: Descriptor,
        offset: FileSize,
        length: FileSize,
        advice: Advice,
    ) -> Result<(), ErrorCode> {
        filesystem::advise(this, offset, length, advice).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_sync_data(&self, this: Descriptor) -> Result<(), ErrorCode> {
        filesystem::sync_data(this).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_get_flags(&self, this: Descriptor) -> Result<DescriptorFlags, ErrorCode> {
        filesystem::get_flags(this).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_get_type(&self, this: Descriptor) -> Result<DescriptorType, ErrorCode> {
        filesystem::get_type(this).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_set_size(&self, this: Descriptor, size: FileSize) -> Result<(), ErrorCode> {
        filesystem::set_size(this, size).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_set_times(
        &self,
        this: Descriptor,
        data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> Result<(), ErrorCode> {
        filesystem::set_times(this, data_access_timestamp, data_modification_timestamp)
            .map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_read(
        &self,
        this: Descriptor,
        length: FileSize,
        offset: FileSize,
    ) -> Result<(Vec<u8>, bool), ErrorCode> {
        filesystem::read(this, length, offset).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_write(
        &self,
        this: Descriptor,
        buffer: Vec<u8>,
        offset: FileSize,
    ) -> Result<FileSize, ErrorCode> {
        filesystem::write(this, &buffer, offset).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_read_directory(
        &self,
        this: Descriptor,
    ) -> Result<DirectoryEntryStream, ErrorCode> {
        filesystem::read_directory(this).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_sync(&self, this: Descriptor) -> Result<(), ErrorCode> {
        filesystem::sync(this).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_create_directory_at(
        &self,
        this: Descriptor,
        path: String,
    ) -> Result<(), ErrorCode> {
        filesystem::create_directory_at(this, &path).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_stat(
        &self,
        this: Descriptor,
        path_flags: PathFlags,
        path: String,
    ) -> Result<DescriptorStat, ErrorCode> {
        filesystem::stat(this, path_flags, &path).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_stat_open_directory(
        &self,
        this: Descriptor,
        path_flags: PathFlags,
        path: String,
    ) -> Result<Descriptor, ErrorCode> {
        filesystem::stat_open_directory(this, path_flags, &path).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_link(
        &self,
        this: Descriptor,
        old_path_flags: PathFlags,
        old_path: String,
        new_descriptor: Descriptor,
        new_path: String,
    ) -> Result<(), ErrorCode> {
        filesystem::link(this, old_path_flags, &old_path, new_descriptor, &new_path)
            .map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_open_at(
        &self,
        this: Descriptor,
        _path_flags: PathFlags,
        path: String,
        open_flags: OpenFlags,
        flags: DescriptorFlags,
    ) -> Result<Descriptor, ErrorCode> {
        match filesystem::open_at(this, &path, open_flags as u32, flags as u32) {
            Ok((fd, _)) => Ok(fd),
            Err(e) => Err(ErrorCode::from(e.errno)),
        }
    }

    pub fn filesystem_readlink_at(
        &self,
        this: Descriptor,
        path: String,
    ) -> Result<String, ErrorCode> {
        filesystem::readlink_at(this, &path).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_remove_directory_at(
        &self,
        this: Descriptor,
        path: String,
    ) -> Result<(), ErrorCode> {
        filesystem::remove_directory_at(this, &path).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_rename_at(
        &self,
        this: Descriptor,
        old_path: String,
        new_descriptor: Descriptor,
        new_path: String,
    ) -> Result<(), ErrorCode> {
        filesystem::rename_at(this, &old_path, new_descriptor, &new_path)
            .map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_symlink_at(
        &self,
        this: Descriptor,
        old_path: String,
        new_path: String,
    ) -> Result<(), ErrorCode> {
        filesystem::symlink_at(this, &old_path, &new_path).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_unlink_file_at(
        &self,
        this: Descriptor,
        path: String,
    ) -> Result<(), ErrorCode> {
        filesystem::unlink_file_at(this, &path).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_is_same_object(&self, this: Descriptor, other: Descriptor) -> bool {
        filesystem::is_same_object(this, other).unwrap_or(false)
    }

    pub fn filesystem_metadata_hash(
        &self,
        this: Descriptor,
    ) -> Result<MetadataHashValue, ErrorCode> {
        filesystem::metadata_hash(this).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_metadata_hash_at(
        &self,
        this: Descriptor,
        path_flags: PathFlags,
        path: String,
    ) -> Result<MetadataHashValue, ErrorCode> {
        filesystem::metadata_hash_at(this, path_flags, &path).map_err(wasi_error_to_error_code)
    }

    pub fn filesystem_drop_descriptor(&self, this: Descriptor) {
        let _ = filesystem::drop_descriptor(this);
    }

    // WIT bindings for wasi:sockets/network@0.2.0
    pub fn sockets_drop_network(&self, this: Network) {
        let _ = sockets::drop_network(this);
    }

    // WIT bindings for wasi:sockets/instance-network@0.2.0
    pub fn sockets_instance_network(&self) -> Network {
        sockets::instance_network()
    }

    // WIT bindings for wasi:sockets/tcp@0.2.0
    pub fn sockets_start_bind(
        &self,
        this: TcpSocket,
        network: Network,
        local_address: IpSocketAddress,
    ) -> Result<(), ErrorCode> {
        sockets::start_bind(this, network, local_address).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_finish_bind(&self, this: TcpSocket) -> Result<(), ErrorCode> {
        sockets::finish_bind(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_start_connect(
        &self,
        this: TcpSocket,
        network: Network,
        remote_address: IpSocketAddress,
    ) -> Result<(), ErrorCode> {
        sockets::start_connect(this, network, remote_address).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_finish_connect(
        &self,
        this: TcpSocket,
    ) -> Result<(InputStream, OutputStream), ErrorCode> {
        sockets::finish_connect(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_start_listen(&self, this: TcpSocket) -> Result<(), ErrorCode> {
        sockets::start_listen(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_finish_listen(&self, this: TcpSocket) -> Result<(), ErrorCode> {
        sockets::finish_listen(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_accept(
        &self,
        this: TcpSocket,
    ) -> Result<Option<(TcpSocket, InputStream, OutputStream)>, ErrorCode> {
        sockets::accept_tcp(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_local_address(&self, this: TcpSocket) -> Result<IpSocketAddress, ErrorCode> {
        sockets::local_address(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_remote_address(&self, this: TcpSocket) -> Result<IpSocketAddress, ErrorCode> {
        sockets::remote_address(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_is_listening(&self, this: TcpSocket) -> bool {
        sockets::is_listening(this)
    }

    pub fn sockets_address_family(&self, this: TcpSocket) -> IpAddressFamily {
        sockets::address_family(this)
    }

    pub fn sockets_set_listen_backlog_size(
        &self,
        this: TcpSocket,
        value: u64,
    ) -> Result<(), ErrorCode> {
        sockets::set_listen_backlog_size(this, value).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_keep_alive_enabled(&self, this: TcpSocket) -> Result<bool, ErrorCode> {
        sockets::keep_alive_enabled(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_set_keep_alive_enabled(
        &self,
        this: TcpSocket,
        value: bool,
    ) -> Result<(), ErrorCode> {
        sockets::set_keep_alive_enabled(this, value).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_keep_alive_idle_time(&self, this: TcpSocket) -> Result<WasiDuration, ErrorCode> {
        // Convert core::time::Duration to WasiDuration (u64 nanoseconds)
        sockets::keep_alive_idle_time(this)
            .map(|dur| dur.as_nanos() as u64)
            .map_err(|_| 1u32)
    }

    pub fn sockets_set_keep_alive_idle_time(
        &self,
        this: TcpSocket,
        value: WasiDuration,
    ) -> Result<(), ErrorCode> {
        // Convert WasiDuration (u64 nanoseconds) to core::time::Duration
        let duration = core::time::Duration::from_nanos(value);
        sockets::set_keep_alive_idle_time(this, duration).map_err(|_| 1u32)
    }

    pub fn sockets_keep_alive_interval(&self, this: TcpSocket) -> Result<WasiDuration, ErrorCode> {
        // Convert core::time::Duration to WasiDuration (u64 nanoseconds)
        sockets::keep_alive_interval(this)
            .map(|dur| dur.as_nanos() as u64)
            .map_err(|_| 1u32)
    }

    pub fn sockets_set_keep_alive_interval(
        &self,
        this: TcpSocket,
        value: WasiDuration,
    ) -> Result<(), ErrorCode> {
        // Convert WasiDuration (u64 nanoseconds) to core::time::Duration
        let duration = core::time::Duration::from_nanos(value);
        sockets::set_keep_alive_interval(this, duration).map_err(|_| 1u32)
    }

    pub fn sockets_keep_alive_count(&self, this: TcpSocket) -> Result<u32, ErrorCode> {
        sockets::keep_alive_count(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_set_keep_alive_count(
        &self,
        this: TcpSocket,
        value: u32,
    ) -> Result<(), ErrorCode> {
        sockets::set_keep_alive_count(this, value).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_hop_limit(&self, this: TcpSocket) -> Result<u8, ErrorCode> {
        sockets::hop_limit(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_set_hop_limit(&self, this: TcpSocket, value: u8) -> Result<(), ErrorCode> {
        sockets::set_hop_limit(this, value).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_receive_buffer_size(&self, this: TcpSocket) -> Result<u64, ErrorCode> {
        sockets::receive_buffer_size(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_set_receive_buffer_size(
        &self,
        this: TcpSocket,
        value: u64,
    ) -> Result<(), ErrorCode> {
        sockets::set_receive_buffer_size(this, value).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_send_buffer_size(&self, this: TcpSocket) -> Result<u64, ErrorCode> {
        sockets::send_buffer_size(this).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_set_send_buffer_size(
        &self,
        this: TcpSocket,
        value: u64,
    ) -> Result<(), ErrorCode> {
        sockets::set_send_buffer_size(this, value).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_subscribe(&self, this: TcpSocket) -> Pollable {
        sockets::subscribe(this).unwrap_or(1u32)
    }

    pub fn sockets_shutdown(
        &self,
        this: TcpSocket,
        shutdown_type: ShutdownType,
    ) -> Result<(), ErrorCode> {
        sockets::shutdown_tcp(this, shutdown_type).map_err(wasi_error_to_error_code)
    }

    pub fn sockets_drop_tcp_socket(&self, this: TcpSocket) {
        let _ = sockets::drop_tcp_socket(this);
    }

    // WIT bindings for wasi:cli/environment@0.2.0
    pub fn cli_get_environment(&self) -> Vec<(String, String)> {
        cli::get_environment().unwrap_or_default()
    }

    pub fn cli_get_arguments(&self) -> Vec<String> {
        cli::get_arguments().unwrap_or_default()
    }

    pub fn cli_initial_cwd(&self) -> Option<String> {
        cli::initial_cwd().ok()
    }

    // WIT bindings for wasi:cli/exit@0.2.0
    pub fn cli_exit(&self, status: Result<(), ()>) -> ! {
        match status {
            Ok(()) => cli::exit_with_code(0),
            Err(()) => cli::exit_with_code(1),
        }
    }

    // WIT bindings for wasi:cli/stdin@0.2.0
    pub fn cli_get_stdin(&self) -> InputStream {
        cli::get_stdin()
    }

    // WIT bindings for wasi:cli/stdout@0.2.0
    pub fn cli_get_stdout(&self) -> OutputStream {
        cli::get_stdout()
    }

    // WIT bindings for wasi:cli/stderr@0.2.0
    pub fn cli_get_stderr(&self) -> OutputStream {
        cli::get_stderr()
    }

    // WIT bindings for wasi:cli/terminal-stdin@0.2.0
    pub fn cli_get_terminal_stdin(&self) -> Option<TerminalInput> {
        cli::get_terminal_stdin()
    }

    // WIT bindings for wasi:cli/terminal-stdout@0.2.0
    pub fn cli_get_terminal_stdout(&self) -> Option<TerminalOutput> {
        cli::get_terminal_stdout()
    }

    // WIT bindings for wasi:cli/terminal-stderr@0.2.0
    pub fn cli_get_terminal_stderr(&self) -> Option<TerminalOutput> {
        cli::get_terminal_stderr()
    }

    // WIT bindings for wasi:http/types@0.2.0
    pub fn http_drop_fields(&self, fields: Fields) {
        http::drop_fields(fields)
    }

    pub fn http_new_fields(&self) -> Fields {
        http::new_fields_v2()
    }

    pub fn http_fields_get(&self, fields: Fields, name: String) -> Vec<Vec<u8>> {
        http::fields_get(fields, &name)
    }

    pub fn http_fields_has(&self, fields: Fields, name: String) -> bool {
        http::fields_has(fields, &name)
    }

    pub fn http_fields_set(
        &self,
        fields: Fields,
        name: String,
        value: Vec<Vec<u8>>,
    ) -> Result<(), HeaderError> {
        http::fields_set(fields, &name, &value)
    }

    pub fn http_fields_delete(&self, fields: Fields, name: String) -> Result<(), HeaderError> {
        http::fields_delete(fields, &name)
    }

    pub fn http_fields_append(
        &self,
        fields: Fields,
        name: String,
        value: Vec<u8>,
    ) -> Result<(), HeaderError> {
        http::fields_append(fields, &name, &value)
    }

    pub fn http_fields_entries(&self, fields: Fields) -> Vec<(String, Vec<u8>)> {
        http::fields_entries(fields)
    }

    pub fn http_fields_clone(&self, fields: Fields) -> Fields {
        http::fields_clone(fields)
    }

    pub fn http_finish_incoming_stream(
        &self,
        s: IncomingStream,
    ) -> Result<Option<Trailers>, ErrorCode> {
        option_u32_result_convert(http::finish_incoming_stream(s))
    }

    pub fn http_finish_outgoing_stream(
        &self,
        s: OutgoingStream,
        trailers: Option<Trailers>,
    ) -> Result<(), ErrorCode> {
        unit_result_convert(http::finish_outgoing_stream(s, trailers))
    }

    pub fn http_incoming_request_method(&self, this: IncomingRequest) -> Method {
        http::incoming_request_method(this)
    }

    pub fn http_incoming_request_path_with_query(&self, this: IncomingRequest) -> Option<String> {
        http::incoming_request_path_with_query(this)
    }

    pub fn http_incoming_request_scheme(&self, this: IncomingRequest) -> Option<Scheme> {
        http::incoming_request_scheme(this)
    }

    pub fn http_incoming_request_authority(&self, this: IncomingRequest) -> Option<String> {
        http::incoming_request_authority(this)
    }

    pub fn http_incoming_request_headers(&self, this: IncomingRequest) -> Headers {
        http::incoming_request_headers(this)
    }

    pub fn http_incoming_request_consume(
        &self,
        this: IncomingRequest,
    ) -> Result<IncomingStream, ()> {
        option_u32_result_wasi_to_unit(http::incoming_request_consume(this))
    }

    pub fn http_new_outgoing_request(&self, headers: Headers) -> OutgoingRequest {
        http::new_outgoing_request(headers)
    }

    pub fn http_outgoing_request_body(&self, this: OutgoingRequest) -> Result<OutgoingStream, ()> {
        option_u32_result_wasi_to_unit(http::outgoing_request_body(this))
    }

    pub fn http_drop_response_outparam(&self, param: ResponseOutparam) {
        http::drop_response_outparam(param)
    }

    pub fn http_set_response_outparam(
        &self,
        param: ResponseOutparam,
        response: Result<OutgoingResponse, ErrorCode>,
    ) -> Result<(), ()> {
        unit_result_wasi_to_unit(http::set_response_outparam(param, response))
    }

    pub fn http_drop_incoming_request(&self, request: IncomingRequest) {
        http::drop_incoming_request(request)
    }

    pub fn http_drop_outgoing_request(&self, request: OutgoingRequest) {
        http::drop_outgoing_request(request)
    }

    pub fn http_incoming_response_status(&self, this: IncomingResponse) -> StatusCode {
        http::incoming_response_status(this)
    }

    pub fn http_incoming_response_headers(&self, this: IncomingResponse) -> Headers {
        http::incoming_response_headers(this)
    }

    pub fn http_incoming_response_consume(
        &self,
        this: IncomingResponse,
    ) -> Result<IncomingStream, ()> {
        option_u32_result_wasi_to_unit(http::incoming_response_consume(this))
    }

    pub fn http_new_outgoing_response(&self, headers: Headers) -> OutgoingResponse {
        http::new_outgoing_response(headers)
    }

    pub fn http_outgoing_response_status_code(&self, this: OutgoingResponse) -> StatusCode {
        http::outgoing_response_status_code(this)
    }

    pub fn http_outgoing_response_set_status_code(
        &self,
        this: OutgoingResponse,
        status_code: StatusCode,
    ) -> Result<(), ()> {
        unit_result_wasi_to_unit(http::outgoing_response_set_status_code(this, status_code))
    }

    pub fn http_outgoing_response_headers(&self, this: OutgoingResponse) -> Headers {
        http::outgoing_response_headers(this)
    }

    pub fn http_outgoing_response_body(
        &self,
        this: OutgoingResponse,
    ) -> Result<OutgoingStream, ()> {
        option_u32_result_wasi_to_unit(http::outgoing_response_body(this))
    }

    pub fn http_drop_incoming_response(&self, response: IncomingResponse) {
        http::drop_incoming_response(response)
    }

    pub fn http_drop_outgoing_response(&self, response: OutgoingResponse) {
        http::drop_outgoing_response(response)
    }

    pub fn http_drop_future_incoming_response(&self, f: FutureIncomingResponse) {
        http::drop_future_incoming_response(f)
    }

    pub fn http_future_incoming_response_get(
        &self,
        this: FutureIncomingResponse,
    ) -> Option<Result<Result<IncomingResponse, ErrorCode>, ()>> {
        http::future_incoming_response_get(this)
    }

    pub fn http_listen_to_future_incoming_response(
        &self,
        this: FutureIncomingResponse,
    ) -> Pollable {
        http::listen_to_future_incoming_response(this)
    }

    // WIT bindings for wasi:http/outgoing-handler@0.2.0
    pub fn http_handle(
        &self,
        request: OutgoingRequest,
        options: Option<RequestOptions>,
    ) -> Result<FutureIncomingResponse, ErrorCode> {
        http::handle(request, options).map_err(wasi_error_to_error_code)
    }
}

// Default implementation
impl Default for Component {
    fn default() -> Self {
        Self::new()
    }
}
