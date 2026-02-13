// IngestionClient.swift
// Client for sending payloads to the ingestion service

import Foundation
import os.log

/// Client for communicating with the ingestion service via Unix socket
public class IngestionClient {
    
    // MARK: - Configuration
    
    public struct Config {
        /// Path to the Unix socket
        public var socketPath: String = "/tmp/clace-ingestion.sock"
        /// Connection timeout in seconds
        public var timeout: TimeInterval = 5.0
        /// Retry count on failure
        public var retryCount: Int = 3
        /// Delay between retries
        public var retryDelay: TimeInterval = 1.0
        
        public init() {}
    }
    
    // MARK: - Properties
    
    private let logger = Logger(subsystem: "com.clace.ocr", category: "IngestionClient")
    private var config: Config
    
    // MARK: - Initialization
    
    public init(config: Config = Config()) {
        self.config = config
    }
    
    // MARK: - Connection Check
    
    /// Check if the ingestion service is available
    public func isServiceAvailable() -> Bool {
        return FileManager.default.fileExists(atPath: config.socketPath)
    }
    
    // MARK: - Send Payload
    
    /// Send payload to ingestion service
    public func send(_ payload: CapturePayload) async throws -> IngestionResponse {
        let jsonData = try JSONEncoder().encode(payload)
        
        var lastError: Error?
        
        for attempt in 1...config.retryCount {
            do {
                let response = try await sendViaSocket(jsonData)
                logger.info("Payload sent successfully: \(response.action)")
                return response
            } catch {
                lastError = error
                logger.warning("Send attempt \(attempt) failed: \(error.localizedDescription)")
                
                if attempt < config.retryCount {
                    try await Task.sleep(nanoseconds: UInt64(config.retryDelay * 1_000_000_000))
                }
            }
        }
        
        throw lastError ?? IngestionError.connectionFailed(errno: 0)
    }
    
    /// Send multiple payloads
    public func sendBatch(_ payloads: [CapturePayload]) async -> [(payload: CapturePayload, result: Result<IngestionResponse, Error>)] {
        var results: [(CapturePayload, Result<IngestionResponse, Error>)] = []
        
        for payload in payloads {
            do {
                let response = try await send(payload)
                results.append((payload, .success(response)))
            } catch {
                results.append((payload, .failure(error)))
            }
        }
        
        return results
    }
    
    // MARK: - Private Implementation
    
    private func sendViaSocket(_ data: Data) async throws -> IngestionResponse {
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global(qos: .utility).async {
                do {
                    let response = try self.performSocketSend(data)
                    continuation.resume(returning: response)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }
    
    private func performSocketSend(_ data: Data) throws -> IngestionResponse {
        // Create socket
        let socketFD = socket(AF_UNIX, SOCK_STREAM, 0)
        guard socketFD >= 0 else {
            throw IngestionError.socketCreationFailed
        }
        defer { close(socketFD) }
        
        // Set timeout
        var timeout = timeval(
            tv_sec: Int(config.timeout),
            tv_usec: Int32((config.timeout.truncatingRemainder(dividingBy: 1)) * 1_000_000)
        )
        setsockopt(socketFD, SOL_SOCKET, SO_RCVTIMEO, &timeout, socklen_t(MemoryLayout<timeval>.size))
        setsockopt(socketFD, SOL_SOCKET, SO_SNDTIMEO, &timeout, socklen_t(MemoryLayout<timeval>.size))
        
        // Setup address
        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        
        // Copy socket path safely
        let pathBytes = config.socketPath.utf8CString
        _ = pathBytes.withUnsafeBufferPointer { srcBuffer in
            withUnsafeMutableBytes(of: &addr.sun_path) { destBuffer in
                let count = min(srcBuffer.count, destBuffer.count)
                for i in 0..<count {
                    destBuffer[i] = UInt8(bitPattern: srcBuffer[i])
                }
            }
        }
        
        // Connect
        let connectResult = withUnsafePointer(to: &addr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockaddrPtr in
                Darwin.connect(socketFD, sockaddrPtr, socklen_t(MemoryLayout<sockaddr_un>.size))
            }
        }
        
        guard connectResult == 0 else {
            throw IngestionError.connectionFailed(errno: errno)
        }
        
        // Send payload with newline delimiter
        var payload = data
        payload.append(contentsOf: "\n".utf8)
        
        let bytesSent = payload.withUnsafeBytes { ptr in
            Darwin.send(socketFD, ptr.baseAddress, ptr.count, 0)
        }
        
        guard bytesSent == payload.count else {
            throw IngestionError.sendFailed(sent: bytesSent, expected: payload.count)
        }
        
        // Read response
        var responseBuffer = [UInt8](repeating: 0, count: 4096)
        let bytesRead = recv(socketFD, &responseBuffer, responseBuffer.count, 0)
        
        guard bytesRead > 0 else {
            throw IngestionError.noResponse
        }
        
        // Find newline delimiter
        var responseEnd = bytesRead
        for i in 0..<bytesRead {
            if responseBuffer[i] == UInt8(ascii: "\n") {
                responseEnd = i
                break
            }
        }
        
        let responseData = Data(responseBuffer.prefix(responseEnd))
        
        do {
            return try JSONDecoder().decode(IngestionResponse.self, from: responseData)
        } catch {
            // Try to parse as error response
            if let responseStr = String(data: responseData, encoding: .utf8) {
                logger.error("Failed to decode response: \(responseStr)")
            }
            throw IngestionError.decodingFailed(error)
        }
    }
    
    // MARK: - Configuration
    
    /// Update configuration
    public func updateConfig(_ config: Config) {
        self.config = config
    }
}

// MARK: - Errors

public enum IngestionError: Error, LocalizedError {
    case socketCreationFailed
    case connectionFailed(errno: Int32)
    case sendFailed(sent: Int, expected: Int)
    case noResponse
    case decodingFailed(Error)
    case serviceUnavailable
    
    public var errorDescription: String? {
        switch self {
        case .socketCreationFailed:
            return "Failed to create Unix socket"
        case .connectionFailed(let errno):
            return "Failed to connect to ingestion service (errno: \(errno))"
        case .sendFailed(let sent, let expected):
            return "Failed to send data (sent \(sent) of \(expected) bytes)"
        case .noResponse:
            return "No response from ingestion service"
        case .decodingFailed(let error):
            return "Failed to decode response: \(error.localizedDescription)"
        case .serviceUnavailable:
            return "Ingestion service is not available"
        }
    }
}
