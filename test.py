import requests
import concurrent.futures
import time
import datetime

def send_request(url, counter):
    json_data = {
        "id": counter,
        "user_id": "String",
        # "code": "#include <bits/stdc++.h>\nusing namespace std;\nint fib(int n)\n {\nif (n <= 2) return 1;\nreturn fib(n-1)+fib(n-2);\n}\nint main() {\ncout<<fib(42)*fib(41);\n}",
        "code": "#include <bits/stdc++.h>\nusing namespace std;\nlong dp[20000000];\nint main() {\nint a, b;\ncin >> a >> b;\ncout << a+b << endl;\nfor(int i = 0; i < 20000000; i++) dp[i] = 1; while(true);\n}",
        "language": "cpp"
    }
    start_time = time.time()
    response = requests.post(url, json=json_data)
    end_time = time.time()
    return url, response.status_code, end_time - start_time

def concurrency_testing(api_url, num_requests, concurrency_level):
    urls = [api_url] * num_requests
    with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency_level) as executor:
        results = executor.map(send_request, urls, range(1, num_requests + 1))
    return list(results)

def sequential_testing(api_url, num_requests, delay):
    results = []
    for i in range(num_requests):
        result = send_request(api_url, i + 1)
        results.append(result)
        time.sleep(delay)
    return results


if __name__ == "__main__":
    api_url = "http://localhost:3000/submit"
    num_requests = 100
    concurrency_level = 5

    print(datetime.datetime.now())
    
    print(f"Sending {num_requests} requests to {api_url} with concurrency level {concurrency_level}...\n")
    results = concurrency_testing(api_url, num_requests, concurrency_level)

    # print(f"Sending {num_requests} requests to {api_url} sequentially...\n")
    # results = sequential_testing(api_url, num_requests, 0.5)
    
    # Print results
    for url, status_code, response_time in results:
        print(f"URL: {url}, Status Code: {status_code}, Response Time: {response_time:.2f} seconds")
