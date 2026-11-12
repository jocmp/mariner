import Foundation
import Mariner

/// Core's HTTP client. It sends one request and hands back the response as
/// it came, redirects included: core follows them itself, so it can record
/// each one.
nonisolated final class URLSessionClient: HttpClient {
    private let userAgent: String = {
        let version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0"
        return "Mariner/\(version) (+https://jocmp.com)"
    }()

    private let session = URLSession(
        configuration: .default,
        delegate: StopAtRedirects(),
        delegateQueue: nil
    )

    func send(request: HttpRequest) async throws -> HttpResponse {
        guard let url = URL(string: request.url) else {
            throw HttpError.Request(reason: "Not a URL: \(request.url)")
        }
        var urlRequest = URLRequest(url: url, timeoutInterval: 30)
        urlRequest.setValue(userAgent, forHTTPHeaderField: "User-Agent")
        for header in request.headers {
            urlRequest.addValue(header.value, forHTTPHeaderField: header.name)
        }
        do {
            let (data, response) = try await session.data(for: urlRequest)
            guard let response = response as? HTTPURLResponse else {
                throw HttpError.Request(reason: "Not an HTTP response")
            }
            let headers = response.allHeaderFields.map { name, value in
                HttpHeader(name: "\(name)", value: "\(value)")
            }
            return HttpResponse(
                status: UInt16(clamping: response.statusCode),
                headers: headers,
                body: data
            )
        } catch let error as URLError where error.code == .timedOut {
            throw HttpError.Timeout
        } catch let error as URLError {
            throw HttpError.Connect(reason: error.localizedDescription)
        }
    }
}

/// Stops at each redirect, so the response goes back to core. (The
/// completion-handler form: Swift 6.4's compiler crashes on the async one.)
private nonisolated final class StopAtRedirects: NSObject, URLSessionTaskDelegate, Sendable {
    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        willPerformHTTPRedirection response: HTTPURLResponse,
        newRequest request: URLRequest,
        completionHandler: @escaping @Sendable (URLRequest?) -> Void
    ) {
        completionHandler(nil)
    }
}
